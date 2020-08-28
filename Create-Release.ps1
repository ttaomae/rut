cargo build --release
$rut = ".\target\release\rut.exe"

$version = "v$(cargo pkgid rut | &$rut -d# -f2 | &$rut -d: -f2)"
$toolchain = rustup show active-toolchain | &$rut -d' ' -f1 | &$rut -d- -f2-

$package_name = "rut-$version-$toolchain"
$package_directory = ".\target\$package_name"
$archive_path = ".\target\$package_name.zip"

if (Test-Path $package_directory) {
    Write-Error "Package directory $package_directory already exists."
    exit
}

if (Test-Path $archive_path) {
    Write-Error "Package archive $archive_path already exists."
    exit
}

Write-Output "Creating package $package_directory ..."

New-Item -Type Directory $package_directory | Out-Null
Copy-Item -Path README.md -Destination $package_directory
Copy-Item -Path LICENSE.md -Destination $package_directory
Copy-Item -Path $rut -Destination $package_directory

Write-Output "Creating archive $archive_path ..."
Compress-Archive -Path $package_directory -DestinationPath $archive_path

Write-Output "Removing package $package_directory ..."
Remove-Item -Recurse $package_directory
