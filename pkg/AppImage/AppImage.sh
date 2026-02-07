#!/bin/sh

# You might need to restart your pc if sharun doesn't create `AppDir` in this directory (It should create dirs on its own)

set -eu

ARCH="$(uname -m)"
SHARUN="https://raw.githubusercontent.com/pkgforge-dev/Anylinux-AppImages/753b3ba3c77a573f8c2eeb0b397752df2d3235df/useful-tools/quick-sharun.sh"

export ADD_HOOKS="self-updater.bg.hook"
#export UPINFO="gh-releases-zsync|${GITHUB_REPOSITORY%/*}|${GITHUB_REPOSITORY#*/}|latest|*$ARCH.AppImage.zsync"
export OUTNAME=maxima-anylinux-"$ARCH".AppImage
export DESKTOP=./pkg/AppImage/maxima.desktop
export ICON=./maxima-resources/assets/logo.png
export DEPLOY_OPENGL=0
export DEPLOY_VULKAN=0
export DEPLOY_DOTNET=0

#Remove leftovers
rm -rf AppDir dist appinfo

# ADD LIBRARIES
wget --retry-connrefused --tries=30 "$SHARUN" -O ./quick-sharun
chmod +x ./quick-sharun

# Point to binaries
./quick-sharun ./target/$(uname -m)-unknown-linux-musl/release/maxima-bootstrap ./target/$(uname -m)-unknown-linux-musl/release/maxima-cli ./target/$(uname -m)-unknown-linux-musl/release/maxima-tui ./target/$(uname -m)-unknown-linux-musl/release/maxima

# Make AppImage
./quick-sharun --make-appimage

mkdir -p ./dist
mv -v ./*.AppImage* ./dist

echo "All Done!"