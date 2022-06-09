export prefix ?= /usr
sysconfdir ?= /etc
bindir = $(prefix)/bin
libdir = $(prefix)/lib

ID=com.system76.Distinst
BINARY=distinst_v2

TARGET = debug
DEBUG ?= 0

.PHONY = all clean install uninstall vendor

ifeq ($(DEBUG),0)
	TARGET = release
	ARGS += --release
endif

VENDOR ?= 0
ifneq ($(VENDOR),0)
	ARGS += --frozen
endif

TARGET_BIN="$(DESTDIR)$(bindir)/$(ID)"
TARGET_DBUS_CONF="$(DESTDIR)$(sysconfdir)/dbus-1/system.d/distinst-v2.conf"

all: extract-vendor
	cargo build $(ARGS)

clean:
	cargo clean

distclean:
	rm -rf .cargo vendor vendor.tar target

vendor:
	mkdir -p .cargo
	cargo vendor --sync crates/disk-manager/Cargo.toml | head -n -1 > .cargo/config
	echo 'directory = "vendor"' >> .cargo/config
	tar pcf vendor.tar vendor
	rm -rf vendor

extract-vendor:
ifeq ($(VENDOR),1)
	ls
	rm -rf vendor; tar pxf vendor.tar
endif

install:
	install -Dm0755 "target/$(TARGET)/$(BINARY)" "$(TARGET_BIN)"
	install -Dm0644 "data/distinst-v2.conf" "$(TARGET_DBUS_CONF)"

uninstall:
	rm "$(TARGET_BIN)" "$(TARGET_DBUS_CONF)" "$(TARGET_SYSTEMD_SERVICE)"
