RUST_BUILDER ?= cross
APPIMAGETOOL ?= appimagetool

# Build Appimage for BeagleBoardImager GUI
define appimage
	mkdir -p release/linux/$(1)/AppDir/usr/bin
	cp gui/assets/AppRun release/linux/$(1)/AppDir/AppRun
	cp target/$(1)/release/bb-imager-gui release/linux/$(1)/AppDir/usr/bin/
	cp gui/BeagleBoardImager.desktop release/linux/$(1)/AppDir/
	cp gui/icon.png release/linux/$(1)/AppDir/
	ARCH=$(2) $(APPIMAGETOOL) --appimage-extract-and-run release/linux/$(1)/AppDir release/linux/$(1)/BeagleBoardImager.AppImage
	rm -rf release/linux/$(1)/AppDir
endef

# Build Macos dmg for BeagleBoardImager GUI
define dmg
	sed -i'.bak' -e "s/0\.0\.0/${CI_COMMIT_TAG}/g" -e "s/fffffff/${CI_COMMIT_SHA:0:7}/g" gui/assets/packaging/macos/Info.plist
	mkdir -p release/darwin/$(1)/AppDir/BeagleBoardImager.app/Contents/{MacOS,Resources}
	cp gui/assets/packaging/macos/Info.plist release/darwin/$(1)/AppDir/BeagleBoardImager.app/Contents/
	cp gui/assets/icons/icon.icns release/darwin/$(1)/AppDir/BeagleBoardImager.app/Contents/Resources/
	cp target/$(1)/release/bb-imager-gui release/darwin/$(1)/AppDir/BeagleBoardImager.app/Contents/MacOS/
	create-dmg \
		--volname "BeagleBoardImager Installer" \
		--volicon "gui/assets/icons/icon.icns" \
		--window-pos 200 120 \
		--window-size 800 400 \
		--icon-size 100 \
		--icon "BeagleBoardImager.app" 200 190 \
		--hide-extension "BeagleBoardImager.app" \
		--app-drop-link 600 185 \
		--hdiutil-verbose \
		--skip-jenkins \
		"release/darwin/$(1)/BeagleBoardImager.dmg" \
		"release/darwin/$(1)/AppDir/"
	rm -rf release/darwin/$(1)/AppDir/
endef

# Build Executable for BeagleBoardImager CLI
define cli
	mkdir -p release/linux/$(1)
	xz -vc target/$(1)/release/bb-imager-cli > release/linux/$(1)/bb-imager-cli.xz
endef

# Upload linux artifacts
define upload_linux_artifact
	curl --fail-with-body --header "JOB-TOKEN: ${CI_JOB_TOKEN}" --upload-file release/linux/$(1)/BeagleBoardImager.AppImage ${CI_API_V4_URL}/projects/${CI_PROJECT_ID}/packages/generic/bb-imager-gui/${CI_COMMIT_TAG}/$(1).AppImage
	curl --fail-with-body --header "JOB-TOKEN: ${CI_JOB_TOKEN}" --upload-file release/linux/$(1)/bb-imager-cli.xz ${CI_API_V4_URL}/projects/${CI_PROJECT_ID}/packages/generic/bb-imager-cli/${CI_COMMIT_TAG}/$(1).xz
endef

clean:
	rm -rf release

build-windows-x86_64:
	$(info "Building Windows release for x86_64")
	$(RUST_BUILDER) build --release --target x86_64-pc-windows-gnu

build-linux-x86_64:
	$(info "Building Linux release for x86_64")
	$(RUST_BUILDER) build --release --target x86_64-unknown-linux-gnu

build-linux-aarch64:
	$(info "Building Linux release for aarch64")
	$(RUST_BUILDER) build --release --target aarch64-unknown-linux-gnu

build-linux-arm:
	$(info "Building Linux release for arm")
	$(RUST_BUILDER) build --release --target armv7-unknown-linux-gnueabihf

build-darwin-x86_64:
	$(info "Building MacOS release for x86_64")
	$(RUST_BUILDER) build --release --target x86_64-apple-darwin

build-darwin-aarch64:
	$(info "Building MacOS release for aarch64")
	$(RUST_BUILDER) build --release --target aarch64-apple-darwin

release-windows-x86_64: build-windows-x86_64
	$(info "Generating Windows release for x86_64")
	mkdir -p release/windows/x86_64-pc-windows-gnu
	zip -j release/windows/x86_64-pc-windows-gnu/bb-imager-cli.zip target/x86_64-pc-windows-gnu/release/bb-imager-cli.exe
	zip -j release/windows/x86_64-pc-windows-gnu/bb-imager-gui.zip target/x86_64-pc-windows-gnu/release/bb-imager-gui.exe

release-linux-gui-appimage-x86_64: build-linux-x86_64
	$(info "Generating Linux Appimage release for x86_64")
	$(call appimage,x86_64-unknown-linux-gnu,x86_64)

release-linux-gui-appimage-aarch64: build-linux-aarch64
	$(info "Generating Linux Appimage release for aarch64")
	$(call appimage,aarch64-unknown-linux-gnu,aarch64)

release-linux-gui-appimage-arm: build-linux-arm
	$(info "Generating Linux Appimage release for aarch64")
	$(call appimage,armv7-unknown-linux-gnueabihf,armhf)

release-linux-cli-x86_64: build-linux-x86_64
	$(info "Generating Linux CLI release for x86_64")
	$(call cli,x86_64-unknown-linux-gnu)

release-linux-cli-aarch64: build-linux-aarch64
	$(info "Generating Linux CLI release for x86_64")
	$(call cli,aarch64-unknown-linux-gnu)

release-linux-cli-arm: build-linux-arm
	$(info "Generating Linux CLI release for x86_64")
	$(call cli,armv7-unknown-linux-gnueabihf)

release-darwin-cli-x86_64: build-darwin-x86_64
	$(info "Generating MacOS CLI release for x86_64")
	mkdir -p release/darwin/x86_64-apple-darwin
	zip -j release/darwin/x86_64-apple-darwin/bb-imager-cli.zip target/x86_64-apple-darwin/release/bb-imager-cli

release-darwin-gui-x86_64: build-darwin-x86_64
	$(info "Generating MacOS GUI release for x86_64")
	$(call dmg,x86_64-apple-darwin)

release-darwin-x86_64: release-darwin-cli-x86_64 release-darwin-gui-x86_64

release-darwin-cli-aarch64: build-darwin-aarch64
	$(info "Generating MacOS CLI release for aarch64")
	mkdir -p release/darwin/aarch64-apple-darwin
	zip -j release/darwin/aarch64-apple-darwin/bb-imager-cli.zip target/aarch64-apple-darwin/release/bb-imager-cli

release-darwin-gui-aarch64: build-darwin-aarch64
	$(info "Generating MacOS GUI release for aarch64")
	$(call dmg,aarch64-apple-darwin)

release-darwin-aarch64: release-darwin-cli-aarch64 release-darwin-gui-aarch64

release-linux-x86_64: release-linux-cli-x86_64 release-linux-gui-appimage-x86_64

release-linux-aarch64: release-linux-cli-aarch64 release-linux-gui-appimage-aarch64

release-linux-arm: release-linux-cli-arm release-linux-gui-appimage-arm

release-linux: release-linux-x86_64 release-linux-aarch64 release-linux-arm

# TODO: Add aarch64 windows.
release-windows: release-windows-x86_64

release-darwin: release-darwin-x86_64 release-darwin-aarch64

upload-artifacts-windows-x86_64:
	$(info "Upload Windows x86_64 artifacts")
	curl --fail-with-body --header "JOB-TOKEN: ${CI_JOB_TOKEN}" --upload-file release/windows/x86_64-pc-windows-gnu/bb-imager-gui.zip ${CI_API_V4_URL}/projects/${CI_PROJECT_ID}/packages/generic/bb-imager-gui/${CI_COMMIT_TAG}/x86_64-pc-windows-gnu.zip
	curl --fail-with-body --header "JOB-TOKEN: ${CI_JOB_TOKEN}" --upload-file release/windows/x86_64-pc-windows-gnu/bb-imager-cli.zip ${CI_API_V4_URL}/projects/${CI_PROJECT_ID}/packages/generic/bb-imager-cli/${CI_COMMIT_TAG}/x86_64-pc-windows-gnu.zip

upload-artifacts-linux-x86_64:
	$(info "Upload Linux x86_64 artifacts")
	$(call upload_linux_artifact,x86_64-unknown-linux-gnu)

upload-artifacts-linux-aarch64:
	$(info "Upload Linux aarch64 artifacts")
	$(call upload_linux_artifact,aarch64-unknown-linux-gnu)

upload-artifacts-linux-arm:
	$(info "Upload Linux arm artifacts")
	$(call upload_linux_artifact,armv7-unknown-linux-gnueabihf)
	curl --fail-with-body --header "JOB-TOKEN: ${CI_JOB_TOKEN}" --upload-file release/darwin/x86_64-apple-darwin/bb-imager-cli.zip ${CI_API_V4_URL}/projects/${CI_PROJECT_ID}/packages/generic/bb-imager-cli/${CI_COMMIT_TAG}/x86_64-apple-darwin.zip

upload-artifacts-darwin-x86_64:
	$(info "Upload MacOS x86_64 artifacts")
	curl --fail-with-body --header "JOB-TOKEN: ${CI_JOB_TOKEN}" --upload-file release/darwin/x86_64-apple-darwin/bb-imager-cli.zip ${CI_API_V4_URL}/projects/${CI_PROJECT_ID}/packages/generic/bb-imager-cli/${CI_COMMIT_TAG}/x86_64-apple-darwin.zip

upload-artifacts-darwin-aarch64:
	$(info "Upload MacOS aarch64 artifacts")
	curl --fail-with-body --header "JOB-TOKEN: ${CI_JOB_TOKEN}" --upload-file release/darwin/aarch64-apple-darwin/bb-imager-cli.zip ${CI_API_V4_URL}/projects/${CI_PROJECT_ID}/packages/generic/bb-imager-cli/${CI_COMMIT_TAG}/aarch64-apple-darwin.zip

upload-artifacts-linux: upload-artifacts-linux-x86_64 upload-artifacts-linux-aarch64 upload-artifacts-linux-arm

upload-artifacts-windows: upload-artifacts-windows-x86_64

upload-artifacts-darwin: upload-artifacts-darwin-x86_64 upload-artifacts-darwin-aarch64

upload-artifacts: upload-artifacts-windows upload-artifacts-linux upload-artifacts-darwin

