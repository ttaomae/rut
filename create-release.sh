#!/bin/bash

cargo build --release
rut=./target/release/rut

version="v$(cargo pkgid rut | $rut -d# -f2 | $rut -d: -f2)"
toolchain=$(rustup show active-toolchain | $rut -d' ' -f1 | $rut -d- -f2-)

package_name="rut-$version-$toolchain"
package_directory="./target/$package_name"
archive_path="./target/$package_name.tar.gz"

if [[ -d $package_directory ]]; then
    echo "Package directory $package_directory already exists." 1>&2
    exit
fi

if [[ -f $archive_path ]]; then
    echo "Package archive $archive_path already exists." 1>&2
    exit
fi

echo "Creating package $package_directory ..."

mkdir $package_directory
cp README.md $package_directory
cp LICENSE.md $package_directory
cp $rut $package_directory

echo "Creating archive $archive_path ..."
tar -czf $archive_path $package_directory

echo "Removing package $package_directory ..."
rm --recursive $package_directory
