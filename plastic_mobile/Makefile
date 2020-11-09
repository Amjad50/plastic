.DEFAULT_GOAL := help
PROJECTNAME=$(shell basename "$(PWD)")
SOURCES=$(sort $(wildcard ./src/*.rs ./src/**/*.rs))

OS_NAME=$(shell uname | tr '[:upper:]' '[:lower:]')
PATH := $(ANDROID_NDK_HOME)/toolchains/llvm/prebuilt/$(OS_NAME)-x86_64/bin:$(PATH)

ANDROID_AARCH64_LINKER=$(ANDROID_NDK_HOME)/toolchains/llvm/prebuilt/$(OS_NAME)-x86_64/bin/aarch64-linux-android29-clang
ANDROID_ARMV7_LINKER=$(ANDROID_NDK_HOME)/toolchains/llvm/prebuilt/$(OS_NAME)-x86_64/bin/armv7a-linux-androideabi29-clang
ANDROID_I686_LINKER=$(ANDROID_NDK_HOME)/toolchains/llvm/prebuilt/$(OS_NAME)-x86_64/bin/i686-linux-android29-clang
ANDROID_X86_64_LINKER=$(ANDROID_NDK_HOME)/toolchains/llvm/prebuilt/$(OS_NAME)-x86_64/bin/x86_64-linux-android29-clang

SHELL := /bin/bash

# ##############################################################################
# # GENERAL
# ##############################################################################

.PHONY: help
help: makefile
	@echo
	@echo " Available actions in "$(PROJECTNAME)":"
	@echo
	@sed -n 's/^##//p' $< | column -t -s ':' |  sed -e 's/^/ /'
	@echo

## init: Install missing dependencies.
.PHONY: init
init:
	rustup target add aarch64-linux-android armv7-linux-androideabi i686-linux-android x86_64-linux-android
	cargo install cbindgen
	flutter pub get

## :

# ##############################################################################
# # RECIPES
# ##############################################################################

## android: Compile the android targets (arm64, armv7 and i686)
debug: symlink_debug dart_bindgen

release: symlink_release dart_bindgen

symlink_debug: target/aarch64-linux-android/debug/libplastic_mobile.so target/armv7-linux-androideabi/debug/libplastic_mobile.so target/i686-linux-android/debug/libplastic_mobile.so target/x86_64-linux-android/debug/libplastic_mobile.so
	ln -r -f -s ./target/aarch64-linux-android/debug/libplastic_mobile.so ./android/app/src/main/jniLibs/arm64-v8a/libplastic_mobile.so
	ln -r -f -s ./target/armv7-linux-androideabi/debug/libplastic_mobile.so ./android/app/src/main/jniLibs/armeabi-v7a/libplastic_mobile.so
	ln -r -f -s ./target/i686-linux-android/debug/libplastic_mobile.so ./android/app/src/main/jniLibs/x86/libplastic_mobile.so
	ln -r -f -s ./target/x86_64-linux-android/debug/libplastic_mobile.so ./android/app/src/main/jniLibs/x86_64/libplastic_mobile.so

target/aarch64-linux-android/debug/libplastic_mobile.so: $(SOURCES) ndk-home
	CC_aarch64_linux_android=$(ANDROID_AARCH64_LINKER) \
	CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER=$(ANDROID_AARCH64_LINKER) \
		cargo build --target aarch64-linux-android # --release

target/armv7-linux-androideabi/debug/libplastic_mobile.so: $(SOURCES) ndk-home
	CC_armv7_linux_androideabi=$(ANDROID_ARMV7_LINKER) \
	CARGO_TARGET_ARMV7_LINUX_ANDROIDEABI_LINKER=$(ANDROID_ARMV7_LINKER) \
		cargo build --target armv7-linux-androideabi # --release

target/i686-linux-android/debug/libplastic_mobile.so: $(SOURCES) ndk-home
	CC_i686_linux_android=$(ANDROID_I686_LINKER) \
	CARGO_TARGET_I686_LINUX_ANDROID_LINKER=$(ANDROID_I686_LINKER) \
		cargo  build --target i686-linux-android # --release

target/x86_64-linux-android/debug/libplastic_mobile.so: $(SOURCES) ndk-home
	CC_x86_64_linux_android=$(ANDROID_X86_64_LINKER) \
	CARGO_TARGET_X86_64_LINUX_ANDROID_LINKER=$(ANDROID_X86_64_LINKER) \
		cargo build --target x86_64-linux-android # --release


symlink_release: target/aarch64-linux-android/release/libplastic_mobile.so target/armv7-linux-androideabi/release/libplastic_mobile.so target/i686-linux-android/release/libplastic_mobile.so target/x86_64-linux-android/release/libplastic_mobile.so
	ln -r -f -s ./target/aarch64-linux-android/release/libplastic_mobile.so ./android/app/src/main/jniLibs/arm64-v8a/libplastic_mobile.so
	ln -r -f -s ./target/armv7-linux-androideabi/release/libplastic_mobile.so ./android/app/src/main/jniLibs/armeabi-v7a/libplastic_mobile.so
	ln -r -f -s ./target/i686-linux-android/release/libplastic_mobile.so ./android/app/src/main/jniLibs/x86/libplastic_mobile.so
	ln -r -f -s ./target/x86_64-linux-android/release/libplastic_mobile.so ./android/app/src/main/jniLibs/x86_64/libplastic_mobile.so
		
target/aarch64-linux-android/release/libplastic_mobile.so: $(SOURCES) ndk-home
	CC_aarch64_linux_android=$(ANDROID_AARCH64_LINKER) \
	CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER=$(ANDROID_AARCH64_LINKER) \
		cargo build --target aarch64-linux-android --release

target/armv7-linux-androideabi/release/libplastic_mobile.so: $(SOURCES) ndk-home
	CC_armv7_linux_androideabi=$(ANDROID_ARMV7_LINKER) \
	CARGO_TARGET_ARMV7_LINUX_ANDROIDEABI_LINKER=$(ANDROID_ARMV7_LINKER) \
		cargo build --target armv7-linux-androideabi --release

target/i686-linux-android/release/libplastic_mobile.so: $(SOURCES) ndk-home
	CC_i686_linux_android=$(ANDROID_I686_LINKER) \
	CARGO_TARGET_I686_LINUX_ANDROID_LINKER=$(ANDROID_I686_LINKER) \
		cargo  build --target i686-linux-android --release

target/x86_64-linux-android/release/libplastic_mobile.so: $(SOURCES) ndk-home
	CC_x86_64_linux_android=$(ANDROID_X86_64_LINKER) \
	CARGO_TARGET_X86_64_LINUX_ANDROID_LINKER=$(ANDROID_X86_64_LINKER) \
		cargo build --target x86_64-linux-android --release

dart_bindgen: target/binding.h
	flutter pub run ffigen

.PHONY: ndk-home
ndk-home:
	@if [ ! -d "${ANDROID_NDK_HOME}" ] ; then \
		echo "Error: Please, set the ANDROID_NDK_HOME env variable to point to your NDK folder" ; \
		exit 1 ; \
	fi
## :

# ##############################################################################
# # OTHER
# ##############################################################################

## clean:
.PHONY: clean
clean:
	cargo clean

## test:
.PHONY: test
test:
	cargo test
