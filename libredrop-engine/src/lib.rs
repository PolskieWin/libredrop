use jni::objects::{JClass, JObjectArray, JString};
use jni::sys::{jboolean, jint, jstring, JNI_FALSE, JNI_TRUE};
use jni::JNIEnv;
use libredrop_core::discovery::DiscoveryManager;
use libredrop_core::models::DropConfig;
use libredrop_core::transfer::TransferManager;
use libredrop_core::utils::get_local_ip;
use libredrop_core::web_drop::WebDropServer;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use tokio::runtime::Runtime;

static INITIALIZED: AtomicBool = AtomicBool::new(false);

struct EngineState {
    runtime: Runtime,
    config: DropConfig,
    discovery: Arc<DiscoveryManager>,
    transfer: Arc<TransferManager>,
    web_server: Arc<Mutex<Option<Arc<WebDropServer>>>>,
}

static ENGINE: Mutex<Option<EngineState>> = Mutex::new(None);

#[no_mangle]
pub extern "system" fn Java_org_libredrop_engine_NativeEngine_nativeGetVersion(
    env: JNIEnv,
    _class: JClass,
) -> jstring {
    let version = "LibreDrop v1.0.0-rs (Zero-Downscale Native P2P)";
    env.new_string(version).unwrap().into_raw()
}

#[no_mangle]
pub extern "system" fn Java_org_libredrop_engine_NativeEngine_nativeGetLocalIp(
    env: JNIEnv,
    _class: JClass,
) -> jstring {
    let ip = get_local_ip().to_string();
    env.new_string(ip).unwrap().into_raw()
}

#[no_mangle]
pub extern "system" fn Java_org_libredrop_engine_NativeEngine_nativeInit(
    mut env: JNIEnv,
    _class: JClass,
    config_json: JString,
) -> jboolean {
    let json_str: String = match env.get_string(&config_json) {
        Ok(s) => s.into(),
        Err(_) => return JNI_FALSE,
    };

    let config: DropConfig = match serde_json::from_str(&json_str) {
        Ok(c) => c,
        Err(_) => DropConfig::default(),
    };

    let rt = match Runtime::new() {
        Ok(r) => r,
        Err(_) => return JNI_FALSE,
    };

    let discovery = Arc::new(DiscoveryManager::new(config.clone()));
    let transfer = Arc::new(TransferManager::new(config.clone()));

    let state = EngineState {
        runtime: rt,
        config,
        discovery,
        transfer,
        web_server: Arc::new(Mutex::new(None)),
    };

    let mut lock = ENGINE.lock().unwrap();
    *lock = Some(state);
    INITIALIZED.store(true, Ordering::SeqCst);

    JNI_TRUE
}

#[no_mangle]
pub extern "system" fn Java_org_libredrop_engine_NativeEngine_nativeStartDiscovery(
    _env: JNIEnv,
    _class: JClass,
) -> jboolean {
    let lock = ENGINE.lock().unwrap();
    if let Some(ref state) = *lock {
        let disc = state.discovery.clone();
        state.runtime.spawn(async move {
            let _ = disc.start().await;
        });
        JNI_TRUE
    } else {
        JNI_FALSE
    }
}

#[no_mangle]
pub extern "system" fn Java_org_libredrop_engine_NativeEngine_nativeStopDiscovery(
    _env: JNIEnv,
    _class: JClass,
) {
    let lock = ENGINE.lock().unwrap();
    if let Some(ref state) = *lock {
        let disc = state.discovery.clone();
        state.runtime.spawn(async move {
            disc.stop().await;
        });
    }
}

#[no_mangle]
pub extern "system" fn Java_org_libredrop_engine_NativeEngine_nativeGetPeers(
    env: JNIEnv,
    _class: JClass,
) -> jstring {
    let lock = ENGINE.lock().unwrap();
    if let Some(ref state) = *lock {
        let disc = state.discovery.clone();
        let peers = state.runtime.block_on(async move {
            disc.get_peers().await
        });
        let json = serde_json::to_string(&peers).unwrap_or_else(|_| "[]".to_string());
        env.new_string(json).unwrap().into_raw()
    } else {
        env.new_string("[]").unwrap().into_raw()
    }
}

#[no_mangle]
pub extern "system" fn Java_org_libredrop_engine_NativeEngine_nativeStartServer(
    _env: JNIEnv,
    _class: JClass,
) -> jboolean {
    let lock = ENGINE.lock().unwrap();
    if let Some(ref state) = *lock {
        let transfer = state.transfer.clone();
        state.runtime.spawn(async move {
            let _ = transfer.start_server().await;
        });
        JNI_TRUE
    } else {
        JNI_FALSE
    }
}

#[no_mangle]
pub extern "system" fn Java_org_libredrop_engine_NativeEngine_nativeStopServer(
    _env: JNIEnv,
    _class: JClass,
) {
    let lock = ENGINE.lock().unwrap();
    if let Some(ref state) = *lock {
        state.transfer.stop_server();
    }
}

#[no_mangle]
pub extern "system" fn Java_org_libredrop_engine_NativeEngine_nativeSendTransfer(
    mut env: JNIEnv,
    _class: JClass,
    target_ip: JString,
    target_port: jint,
    message_str: JString,
    files_array: JObjectArray,
) -> jboolean {
    let ip: String = match env.get_string(&target_ip) {
        Ok(s) => s.into(),
        Err(_) => return JNI_FALSE,
    };

    let msg: Option<String> = if !message_str.is_null() {
        env.get_string(&message_str).ok().map(|s| s.into())
    } else {
        None
    };

    let mut file_paths = Vec::new();
    if !files_array.is_null() {
        let len = match env.get_array_length(&files_array) {
            Ok(l) => l,
            Err(_) => 0,
        };
        for i in 0..len {
            if let Ok(obj) = env.get_object_array_element(&files_array, i) {
                let jstr = JString::from(obj);
                let converted: Result<String, _> = env.get_string(&jstr).map(|s| s.into());
                if let Ok(path_str) = converted {
                    file_paths.push(path_str);
                }
            }
        }
    }

    let lock = ENGINE.lock().unwrap();
    if let Some(ref state) = *lock {
        let config = state.config.clone();
        let port = target_port as u16;
        let (tx, _rx) = tokio::sync::broadcast::channel(32);

        state.runtime.spawn(async move {
            let _ = TransferManager::send_transfer(
                &ip,
                port,
                &config,
                msg,
                file_paths,
                tx,
            )
            .await;
        });
        JNI_TRUE
    } else {
        JNI_FALSE
    }
}

#[no_mangle]
pub extern "system" fn Java_org_libredrop_engine_NativeEngine_nativeStartWebDrop(
    _env: JNIEnv,
    _class: JClass,
    web_port: jint,
) -> jboolean {
    let lock = ENGINE.lock().unwrap();
    if let Some(ref state) = *lock {
        let conf = state.config.clone();
        let server = Arc::new(WebDropServer::new(conf, web_port as u16));
        let server_clone = server.clone();

        state.runtime.spawn(async move {
            let _ = server_clone.start().await;
        });

        let mut ws_lock = state.web_server.lock().unwrap();
        *ws_lock = Some(server);
        JNI_TRUE
    } else {
        JNI_FALSE
    }
}

#[no_mangle]
pub extern "system" fn Java_org_libredrop_engine_NativeEngine_nativeStopWebDrop(
    _env: JNIEnv,
    _class: JClass,
) {
    let lock = ENGINE.lock().unwrap();
    if let Some(ref state) = *lock {
        let mut ws_lock = state.web_server.lock().unwrap();
        if let Some(server) = ws_lock.take() {
            server.stop();
        }
    }
}
