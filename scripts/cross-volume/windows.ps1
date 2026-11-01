param([Parameter(Mandatory=$true)][string]$Probe,[switch]$PreflightOnly)
$ErrorActionPreference='Stop'
$taskRoot=[IO.Path]::GetFullPath((Get-Location).Path)
$runId=[guid]::NewGuid().ToString()
$runDirectory=Join-Path $taskRoot ".context\cross-volume\$runId"
New-Item -ItemType Directory -Path $runDirectory|Out-Null
$image=Join-Path $runDirectory 'destination.vhdx'
$admin=[Security.Principal.WindowsPrincipal]::new([Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
$report=@{version=1;runId=$runId;platform='win32';administrator=$admin;status='not_run';imageRemoved=$false}
$diskpart=Join-Path $env:SystemRoot 'System32\diskpart.exe'
$report.tools=@{powershell=$PSVersionTable.PSVersion.ToString()}
function Save-Report { [IO.File]::WriteAllText((Join-Path $runDirectory 'report.json'),($report|ConvertTo-Json -Depth 30),[Text.UTF8Encoding]::new($false)) }
function Assert-OwnedImage {
  if([IO.Path]::GetFullPath($image) -ne (Join-Path $runDirectory 'destination.vhdx') -or [IO.Path]::GetDirectoryName($image) -ne $runDirectory -or !$runDirectory.StartsWith((Join-Path $taskRoot '.context\cross-volume\'),[StringComparison]::OrdinalIgnoreCase)){throw 'The virtual disk path escaped the owned evidence directory.'}
  if(Test-Path -LiteralPath $image){$item=Get-Item -LiteralPath $image;if($item.PSIsContainer -or ($item.Attributes -band [IO.FileAttributes]::ReparsePoint)){throw 'The owned image is not a regular file.'}}
  $ancestor=Get-Item -LiteralPath $runDirectory
  while($ancestor){
    if($ancestor.Attributes -band [IO.FileAttributes]::ReparsePoint){throw 'Image cleanup refuses a reparse-point ancestor.'}
    if($ancestor.FullName -eq $taskRoot){break}
    $ancestor=$ancestor.Parent
  }
  if(!$ancestor){throw 'The image is not beneath the intended workspace.'}
}
function Get-OwnedDisk {
  Assert-OwnedImage
  $diskImage=Get-DiskImage -ImagePath $image
  if(!$diskImage.Attached -or [IO.Path]::GetFullPath($diskImage.ImagePath) -ne $image){throw 'The created image is not the attached image being inspected.'}
  $disks=@($diskImage|Get-Disk)
  if($disks.Count -ne 1){throw 'The image did not resolve to exactly one owned disk.'}
  $disk=$disks[0]
  if($disk.IsBoot -or $disk.IsSystem -or $disk.Size -ne 512MB -or [int]$disk.BusType -notin @(14,15)){throw 'Refusing a disk without the exact disposable virtual-disk characteristics.'}
  return $disk
}
try {
  Import-Module (Join-Path $env:SystemRoot 'System32\WindowsPowerShell\v1.0\Modules\Storage\Storage.psd1') -ErrorAction Stop
  $report.tools.storageModule=(Get-Module Storage).Version.ToString()
  $report.tools.diskpartVersion=(Get-Item -LiteralPath $diskpart).VersionInfo.FileVersion
  $stream=[IO.File]::OpenRead($diskpart)
  $hasher=[Security.Cryptography.SHA256]::Create()
  try{$report.tools.diskpartSha256=([BitConverter]::ToString($hasher.ComputeHash($stream))).Replace('-','').ToLowerInvariant()}finally{$hasher.Dispose();$stream.Dispose()}
  if($PreflightOnly){$report.status=if($admin){'preflight_available'}else{'unavailable_requires_administrator'};Save-Report;$report|ConvertTo-Json;exit 0}
  if(!$admin){$report.status='unavailable_requires_administrator';throw 'Windows virtual disk tests require an administrator runner; no volume test was executed.'}
  $probePath=[IO.Path]::GetFullPath($Probe)
  if(!$probePath.StartsWith((Join-Path $taskRoot 'target\'),[StringComparison]::OrdinalIgnoreCase) -or !(Test-Path -LiteralPath $probePath -PathType Leaf)){throw 'Use the built task-workspace fixture probe.'}
  Assert-OwnedImage
  if(Test-Path -LiteralPath $image){throw 'A destination image already exists; it will not be reused or overwritten.'}
  if($image -match '[^\x20-\x7e]' -or $image -match '["\r\n]'){throw 'This fixed ASCII creation harness cannot represent the evidence path safely.'}
  $creation=Join-Path $runDirectory 'create-image.txt'
  "create vdisk file=`"$image`" maximum=512 type=expandable`r`nexit`r`n"|Set-Content -LiteralPath $creation -Encoding ASCII
  & $diskpart /s $creation |Set-Content -LiteralPath (Join-Path $runDirectory 'diskpart-create.log')
  if($LASTEXITCODE -ne 0 -or !(Test-Path -LiteralPath $image -PathType Leaf)){throw 'The disposable image could not be created.'}
  Mount-DiskImage -ImagePath $image -NoDriveLetter -Access ReadWrite|Out-Null
  $disk=Get-OwnedDisk
  if($disk.PartitionStyle -ne 'RAW'){throw 'Only the newly created empty RAW image can be initialized.'}
  if($disk.IsOffline){$disk|Set-Disk -IsOffline $false;$disk=Get-OwnedDisk}
  $initialized=$disk|Initialize-Disk -PartitionStyle GPT -PassThru
  $current=Get-OwnedDisk
  if($initialized.Number -ne $current.Number){throw 'The image-to-disk identity changed before partition creation.'}
  $partition=$current|New-Partition -AssignDriveLetter -UseMaximumSize
  $current=Get-OwnedDisk
  if($partition.DiskNumber -ne $current.Number){throw 'The partition does not belong to the created image.'}
  $label='SH-'+$runId.Replace('-','').Substring(0,24)
  $volume=$partition|Format-Volume -FileSystem NTFS -NewFileSystemLabel $label -AllocationUnitSize 4096 -Confirm:$false
  if($volume.FileSystemLabel -ne $label -or $volume.FileSystem -ne 'NTFS' -or !$volume.DriveLetter){throw 'The created volume does not match the owned NTFS fixture.'}
  $parent=Join-Path "$($volume.DriveLetter):\" "stillharbour-cross-volume-$runId"
  New-Item -ItemType Directory -Path $parent|Out-Null
  [IO.File]::WriteAllText((Join-Path $parent '.stillharbour-fixture-owner'),$runId,[Text.UTF8Encoding]::new($false))
  $scope=@{version=1;runId=$runId;image=$image;destinationParent=$parent}|ConvertTo-Json
  [IO.File]::WriteAllText((Join-Path $runDirectory 'scope.json'),$scope,[Text.UTF8Encoding]::new($false))
  $priorRun=$env:STILLHARBOUR_CROSS_VOLUME_RUN
  try{$env:STILLHARBOUR_CROSS_VOLUME_RUN=$runId;$output=& $probePath;if($LASTEXITCODE -ne 0){throw 'The cross-volume probe failed.'}}finally{$env:STILLHARBOUR_CROSS_VOLUME_RUN=$priorRun}
  $output|Set-Content -LiteralPath (Join-Path $runDirectory 'probe.json') -Encoding UTF8
  $receipt=$output|ConvertFrom-Json
  if(!$receipt.distinctVolumes -or !$receipt.sourcesRetained -or !$receipt.destinationOnlyRetained){throw 'The native receipt did not prove the cross-volume fixture.'}
  $report.receipt=$receipt;$report.status='passed'
} catch {$report.error=$_.Exception.Message;if($report.status -eq 'not_run'){$report.status='failed'};throw}
finally {
  try {
  if(Test-Path -LiteralPath $image -PathType Leaf){
    Assert-OwnedImage
    $attached=Get-DiskImage -ImagePath $image -ErrorAction SilentlyContinue
    if($attached -and $attached.Attached){Dismount-DiskImage -ImagePath $image}
    if($report.status -eq 'passed'){
      $detached=Get-DiskImage -ImagePath $image
      if($detached.Attached){throw 'The image remains attached; it will not be removed.'}
      Assert-OwnedImage
      Remove-Item -LiteralPath $image
      $report.imageRemoved=$true
    }
  }
  } catch {$report.cleanupError=$_.Exception.Message;$report.status='cleanup_failed';throw}
  finally {Save-Report}
}
