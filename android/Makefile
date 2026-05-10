# Android SDK & project paths
ANDROID_SDK_ROOT  := $(shell pwd)/sdk
CMDLINE_TOOLS    := $(ANDROID_SDK_ROOT)/cmdline-tools/latest
SDKMANAGER       := $(CMDLINE_TOOLS)/bin/sdkmanager
ANDROID_HOME     := $(ANDROID_SDK_ROOT)
export ANDROID_HOME
export ANDROID_SDK_ROOT

# Gradle (локальная установка)
GRADLE_HOME      := $(HOME)/gradle-8.9
GRADLE           := $(GRADLE_HOME)/bin/gradle
export PATH := $(GRADLE_HOME)/bin:$(PATH)

# SDK versions
PLATFORM_VERSION := 35
BUILD_TOOLS      := 35.0.0

# APK output
APK_DEBUG := app/build/outputs/apk/debug/app-debug.apk
APK_RELEASE := app/build/outputs/apk/release/app-release-unsigned.apk

.PHONY: setup sdk project build release install clean nuke help

# ============================================================================
# 1. Full setup: SDK → проект готов к сборке
# ============================================================================

setup: sdk local.properties
	@echo "✅ All set! Run 'make build' to build APK"

# ============================================================================
# 2. Install Android SDK (command-line only, no Android Studio)
# ============================================================================

sdk: $(SDKMANAGER)
	@echo "📦 Installing Android platform & build tools..."
	yes | $(SDKMANAGER) --install "platforms;android-$(PLATFORM_VERSION)" "build-tools;$(BUILD_TOOLS)" "platform-tools" 2>&1 | tail -5
	@echo "✅ Android SDK ready ($(ANDROID_SDK_ROOT))"

$(SDKMANAGER):
	@echo "⬇ Downloading Android command-line tools..."
	mkdir -p $(CMDLINE_TOOLS)
	cd $(CMDLINE_TOOLS) && curl -sL https://dl.google.com/android/repository/commandlinetools-linux-11076708_latest.zip -o cmdtools.zip && unzip -qo cmdtools.zip && rm cmdtools.zip
	@# Some versions extract a nested cmdline-tools/ dir — flatten it
	@if [ -d "$(CMDLINE_TOOLS)/cmdline-tools" ]; then mv $(CMDLINE_TOOLS)/cmdline-tools/* $(CMDLINE_TOOLS)/ && rmdir $(CMDLINE_TOOLS)/cmdline-tools; fi
	@echo "✅ Command-line tools installed"

local.properties:
	@echo "sdk.dir=$(ANDROID_SDK_ROOT)" > local.properties

# ============================================================================
# 3. Build
# ============================================================================

build: sdk local.properties
	@test -x "$(GRADLE)" || (echo "❌ Gradle not found at $(GRADLE)"; echo "   Install: curl -sL https://services.gradle.org/distributions/gradle-8.9-bin.zip -o /tmp/gradle.zip && unzip -q /tmp/gradle.zip -d $(HOME)/"; exit 1)
	@echo "🔨 Building debug APK..."
	@$(GRADLE) assembleDebug --no-daemon 2>&1
	@echo ""
	@echo "✅ APK built: $(APK_DEBUG)"
	@ls -lh $(APK_DEBUG)

release: local.properties
	@test -x "$(GRADLE)" || (echo "❌ Gradle not found at $(GRADLE)"; echo "   Install: curl -sL https://services.gradle.org/distributions/gradle-8.9-bin.zip -o /tmp/gradle.zip && unzip -q /tmp/gradle.zip -d $(HOME)/"; exit 1)
	@echo "🔨 Building release APK (unsigned)..."
	@$(GRADLE) assembleRelease --no-daemon 2>&1 | tail -30
	@echo "✅ APK built: $(APK_RELEASE)"
	@ls -lh $(APK_RELEASE)

# ============================================================================
# 4. Install on device
# ============================================================================

install: build
	@echo "📲 Installing on device..."
	$(ANDROID_SDK_ROOT)/platform-tools/adb install -r $(APK_DEBUG)

# ============================================================================
# 5. Emulator
# ============================================================================

avd: sdk
	@echo "📱 Creating emulator..."
	$(SDKMANAGER) --install "system-images;android-$(PLATFORM_VERSION);google_apis;x86_64"
	echo "no" | $(CMDLINE_TOOLS)/bin/avdmanager create avd -n pixel7 -k "system-images;android-$(PLATFORM_VERSION);google_apis;x86_64" -d pixel_7

emu:
	$(ANDROID_SDK_ROOT)/emulator/emulator -avd pixel7 -no-snapshot &

# ============================================================================
# 6. Utilities
# ============================================================================

clean:
	-$(GRADLE) clean --no-daemon 2>/dev/null
	-rm -rf app/build .gradle/
	@echo "🧹 Cleaned"

nuke: clean
	rm -rf $(ANDROID_SDK_ROOT) local.properties
	@echo "💣 Everything nuked (except source code)"

tests:
	@$(GRADLE) test --no-daemon 2>&1 | tail -20

help:
	@echo ""
	@echo "  make setup        — full setup: download SDK + project"
	@echo "  make sdk          — download Android SDK (cmdline-tools only)"
	@echo "  make build        — build debug APK"
	@echo "  make release      — build release APK (unsigned)"
	@echo "  make install      — build & install debug APK via adb"
	@echo "  make tests        — run unit tests"
	@echo "  make avd          — create emulator"
	@echo "  make emu          — start emulator"
	@echo "  make clean        — clean build artifacts"
	@echo "  make nuke         — clean + remove SDK"
	@echo ""
