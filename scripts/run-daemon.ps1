param (
    [Parameter(Mandatory=$false)]
    [ValidateSet("start", "stop", "status", "restart")]
    [string]$Action = "start"
)

$DaemonName = "architext-daemon"
$DaemonPath = Join-Path $PSScriptRoot "..\target\debug\architext-daemon.exe"
if (-not (Test-Path $DaemonPath)) {
    $DaemonPath = Join-Path $PSScriptRoot "..\target\release\architext-daemon.exe"
}

function Get-DaemonProcess {
    return Get-Process -Name $DaemonName -ErrorAction SilentlyContinue
}

switch ($Action.ToLower()) {
    "start" {
        $proc = Get-DaemonProcess
        if ($proc) {
            Write-Host "Architext Daemon is already running (PID: $($proc.Id))." -ForegroundColor Yellow
        } else {
            if (-not (Test-Path $DaemonPath)) {
                Write-Host "Error: Daemon binary not found. Run 'cargo build' first." -ForegroundColor Red
                exit 1
            }
            Write-Host "Starting Architext Daemon in background..." -ForegroundColor Green
            Start-Process -FilePath $DaemonPath -WorkingDirectory (Join-Path $PSScriptRoot "..") -RedirectStandardOutput (Join-Path $PSScriptRoot "..\daemon_out.log") -RedirectStandardError (Join-Path $PSScriptRoot "..\daemon_err.log") -WindowStyle Hidden
            Start-Sleep -Seconds 1
            $proc = Get-DaemonProcess
            if ($proc) {
                Write-Host "Architext Daemon started successfully (PID: $($proc.Id))." -ForegroundColor Green
            } else {
                Write-Host "Failed to start Architext Daemon." -ForegroundColor Red
            }
        }
    }
    "stop" {
        $proc = Get-DaemonProcess
        if ($proc) {
            Write-Host "Stopping Architext Daemon (PID: $($proc.Id))..." -ForegroundColor Yellow
            Stop-Process -Name $DaemonName -Force
            Write-Host "Daemon stopped." -ForegroundColor Green
        } else {
            Write-Host "Architext Daemon is not running." -ForegroundColor Yellow
        }
    }
    "status" {
        $proc = Get-DaemonProcess
        if ($proc) {
            Write-Host "Architext Daemon is RUNNING (PID: $($proc.Id))." -ForegroundColor Green
            $activeFile = Join-Path $HOME ".architext_active"
            if (Test-Path $activeFile) {
                $activeProj = Get-Content $activeFile -Raw
                Write-Host "Active Project Directory: $activeProj" -ForegroundColor Cyan
            }
        } else {
            Write-Host "Architext Daemon is STOPPED." -ForegroundColor Red
        }
    }
    "restart" {
        & $MyInvocation.MyCommand.Path -Action stop
        Start-Sleep -Seconds 1
        & $MyInvocation.MyCommand.Path -Action start
    }
}
