# Development-only user-mode server. No system accounts/shares/services.
$ErrorActionPreference = 'Stop'
$fixtureRoot = [System.IO.Path]::GetFullPath($env:STILLHARBOUR_PROTOCOL_SCOPE)
$fixtureTemp = [System.IO.Path]::GetFullPath([System.IO.Path]::GetTempPath())
$fixtureRelative = [System.IO.Path]::GetRelativePath($fixtureTemp, $fixtureRoot)
$fixtureOwner = $env:STILLHARBOUR_PROTOCOL_OWNER
if ([System.IO.Path]::IsPathRooted($fixtureRelative) -or
    $fixtureRelative -eq '..' -or $fixtureRelative.StartsWith('..' + [System.IO.Path]::DirectorySeparatorChar) -or
    [System.IO.Path]::GetFileName($fixtureRoot) -notlike 'stillharbour-m4-protocol-*' -or
    [System.IO.File]::ReadAllText((Join-Path $fixtureRoot 'owner')) -cne $fixtureOwner) {
    throw 'Invalid constructor-owned SMB test scope.'
}
$sharePath = Join-Path $fixtureRoot 'server'
foreach ($directory in @($fixtureRoot, $sharePath)) {
    if ((Get-Item -LiteralPath $directory).Attributes -band [System.IO.FileAttributes]::ReparsePoint) {
        throw 'Linked SMB test scope is not allowed.'
    }
}
$fixturePort = [int]$env:STILLHARBOUR_PROTOCOL_PORT
if ($fixturePort -le 1024 -or $fixturePort -gt 65535) { throw 'Invalid ephemeral SMB port.' }
$fixtureDlls = Join-Path (Split-Path $PSScriptRoot -Parent) '.context/m4-smb-server'
$expectedHashes = @{
    'SMBLibrary.dll' = '1d0dd528c3f7b04dabfde046d7afb131b747e4659a08e53b3ad9f615833d3d11'
    'SMBLibrary.Win32.dll' = '935c2e508a66299e53be807fb7310d1d3a2f1682d7c0d8bcf2f8bdc92cbf3dbb'
}
foreach ($name in @('SMBLibrary.dll', 'SMBLibrary.Win32.dll')) {
    $dllPath = Join-Path $fixtureDlls $name
    if ((Get-FileHash -LiteralPath $dllPath -Algorithm SHA256).Hash.ToLowerInvariant() -cne $expectedHashes[$name]) { throw 'SMB fixture DLL failed its pin.' }
    Add-Type -Path $dllPath
}
$fixtureReferences = @((Join-Path $fixtureDlls 'SMBLibrary.dll'), (Join-Path $fixtureDlls 'SMBLibrary.Win32.dll')) + [System.IO.Directory]::GetFiles((Join-Path $PSHOME 'ref'), '*.dll')
Add-Type -ReferencedAssemblies $fixtureReferences -TypeDefinition @'
using System;
using System.Net;
using SMBLibrary;
using SMBLibrary.Server;
using SMBLibrary.Authentication.GSSAPI;
using SMBLibrary.Authentication.NTLM;
using SMBLibrary.Win32;
public sealed class StillharbourLoopbackSmb : SMBServer {
    public StillharbourLoopbackSmb(SMBShareCollection shares, GSSProvider provider) : base(shares, provider) { }
    public void StartOwned(int port) {
        base.Start(IPAddress.Loopback, SMBTransportType.DirectTCPTransport, port, false, true, true, TimeSpan.FromSeconds(30));
    }
    public static StillharbourLoopbackSmb Create(string root, string password) {
        var auth = new IndependentNTLMAuthenticationProvider(name => String.Equals(name, "fixture", StringComparison.OrdinalIgnoreCase) ? password : null);
        var shares = new SMBShareCollection();
        shares.Add(new FileSystemShare("FIXTURE", new NTDirectoryFileSystem(root)));
        return new StillharbourLoopbackSmb(shares, new GSSProvider(auth));
    }
}
'@
$fixtureServer = [StillharbourLoopbackSmb]::Create($sharePath, $env:STILLHARBOUR_PROTOCOL_TOKEN)
try {
    $fixtureServer.StartOwned($fixturePort)
    while ($true) { Start-Sleep -Milliseconds 500 }
} finally {
    $fixtureServer.Stop()
}
