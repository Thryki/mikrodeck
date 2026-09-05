# Testa se rodar a sequência de inicialização (a mesma que o Maschine 2 faz) COM PRIVILÉGIO
# DE ADMINISTRADOR é o que falta para acordar o aparelho depois de um religamento físico real.
#
# Este script já roda elevado (é chamado via Start-Process -Verb RunAs). Ele espera o
# usuário desconectar e reconectar o cabo, detecta a reconexão sozinho, e então roda
# a sequência: ler strings do dispositivo, ler os 4 feature/input reports, escrever o LED.
#
# Precisa de administrador.

$ErrorActionPreference = 'Continue'
$saida = "$env:USERPROFILE\AppData\Local\Temp\mikrodeck-captura"
New-Item -ItemType Directory -Force -Path $saida | Out-Null
Start-Transcript -Path (Join-Path $saida 'teste-admin.txt') -Force | Out-Null

$envia = "$env:USERPROFILE\Documents\MikroDeck\mikrodeck\spike-hid\target\debug\envia.exe"
$marca = Join-Path $saida 'etapa-admin.txt'
$id = [Security.Principal.WindowsIdentity]::GetCurrent()
$pr = New-Object Security.Principal.WindowsPrincipal($id)
Write-Host "Rodando como administrador: $($pr.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator))"

function DispositivoPresente {
    (Get-PnpDevice -ErrorAction SilentlyContinue | Where-Object {
        $_.InstanceId -match '^USB\\VID_17CC&PID_1700\\' -and $_.Status -eq 'OK'
    }) -ne $null
}

Write-Host "`n=================================================="  -ForegroundColor Yellow
Write-Host " DESCONECTE O CABO USB DO MIKRO AGORA."                -ForegroundColor Yellow
Write-Host " O script detecta sozinho quando ele sair e voltar."    -ForegroundColor Yellow
Write-Host "=================================================="  -ForegroundColor Yellow
Set-Content $marca 'esperando-sair'

$t0 = Get-Date
while ((DispositivoPresente) -and ((Get-Date) - $t0).TotalSeconds -lt 60) {
    Start-Sleep -Milliseconds 300
}
if (-not (DispositivoPresente)) {
    Write-Host "Aparelho desconectado. Agora RECONECTE o cabo." -ForegroundColor Cyan
    Set-Content $marca 'esperando-voltar'
} else {
    Write-Host "Nao detectei a desconexao em 60s. Abortando." -ForegroundColor Red
    Set-Content $marca 'timeout'
    Stop-Transcript | Out-Null
    Start-Sleep -Seconds 5
    exit 1
}

$t0 = Get-Date
while (-not (DispositivoPresente) -and ((Get-Date) - $t0).TotalSeconds -lt 60) {
    Start-Sleep -Milliseconds 300
}
if (DispositivoPresente) {
    Write-Host "Aparelho reconectado. Aguardando 2s para estabilizar..." -ForegroundColor Green
    Set-Content $marca 'reconectado'
    Start-Sleep -Seconds 2
} else {
    Write-Host "Nao detectei a reconexao em 60s. Abortando." -ForegroundColor Red
    Set-Content $marca 'timeout'
    Stop-Transcript | Out-Null
    Start-Sleep -Seconds 5
    exit 1
}

Write-Host "`n=== Rodando a sequencia de inicializacao (elevado), acendendo em azul, 15s ===" -ForegroundColor Cyan
Set-Content $marca 'rodando-init'
& $envia --init 11 15

Set-Content $marca 'fim'
Write-Host "`nPronto." -ForegroundColor Green
Stop-Transcript | Out-Null
Start-Sleep -Seconds 5
