$ErrorActionPreference = "Stop"
$project = "vespers"

Write-Host "============================="
Write-Host "RESUMING CHAPTER 19 (PROSE)"
Write-Host "============================="

Write-Host "Generating Prose for Chapter 19..."
cargo run --bin run-agent -- --project $project prose 19
Write-Host "Reviewing Chapter 19..."
cargo run --bin run-agent -- --project $project review 19

Write-Host "============================="
Write-Host "STARTING CHAPTER 20"
Write-Host "============================="
Write-Host "Planning Chapter 20..."
cargo run --bin run-agent -- --project $project plan 20
Write-Host "Generating Prose for Chapter 20..."
cargo run --bin run-agent -- --project $project prose 20
Write-Host "Reviewing Chapter 20..."
cargo run --bin run-agent -- --project $project review 20

Write-Host "============================="
Write-Host "ALL 20 CHAPTERS COMPLETED!"
Write-Host "============================="
