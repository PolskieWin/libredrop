@echo off
title LibreDrop Receiver Daemon
echo Starting LibreDrop Receiver Daemon...
echo Files will be saved directly into your Downloads folder.
echo.
"%~dp0libredrop.exe" listen --port 53317
pause
