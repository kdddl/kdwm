all: build

.PHONY: build
build:
	$(shell [[ $EUID -eq 0 ]] && echo "build can not be run as root" && exit 1)
	@echo ":: Rebuilding in release mode..."
	@cargo build --release


.PHONY: build-debug
build-debug:
	$(shell [[ $EUID -eq 0 ]] && echo "build can not be run as root" && exit 1)
	@echo ":: Rebuilding in debug mode..."
	@cargo build

.PHONY: install-helpers
install-helpers:
	@echo ":: Installing ./bin..."
	@mkdir -p /usr/local/bin
	@cp -R bin/. /usr/local/bin
	@ls bin | xargs -I {} chmod 755 /usr/local/bin/{}
	@echo ":: Installing ./scripts..."
	@cp -r scripts /usr/local
	@ls scripts | xargs -I {} chmod 755 /usr/local/scripts/{}
	@echo ":: Copying over xsession file..."
	@cp kdwm.desktop /usr/share/xsessions/

.PHONY: install-penrose-release
install-penrose-release:
	@echo ":: Installing release build of kdwm..."
	@mkdir -p /usr/local/bin
	@cp -f target/release/kdwm /usr/local/bin
	@chmod 755 /usr/local/bin/kdwm

.PHONY: install-penrose-debug
install-penrose-debug:
	@echo ":: Installing debug build of kdwm..."
	@strip target/debug/kdwm
	@mkdir -p /usr/local/bin
	@cp -f target/debug/kdwm /usr/local/bin
	@chmod 755 /usr/local/bin/kdwm

.PHONY: install
install: install-penrose-release install-helpers
	@echo ":: Done"
	
.PHONY: install-debug
install-debug: install-penrose-debug install-helpers
	@echo ":: Done"

.PHONY: uninstall
uninstall:
	@echo ":: Removing binaries..."
	@ls bin | xargs -I {} rm -f /usr/local/bin/{}
	@rm -f /usr/local/bin/kdwm
	@echo ":: Removing scripts..."
	@ls scripts | xargs -I {} rm -f /usr/local/scripts/{}
	@echo ":: Done"

.PHONY: update-penrose
update-penrose:
	@echo "Updating to latest version of penrose from GitHub..."
	cargo update -p penrose
