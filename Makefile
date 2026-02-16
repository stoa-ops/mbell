PREFIX    ?= /usr/local
BINDIR    ?= $(PREFIX)/bin
SYSTEMDDIR ?= $(PREFIX)/lib/systemd/user
LICENSEDIR ?= $(PREFIX)/share/licenses/mbell
DOCDIR    ?= $(PREFIX)/share/doc/mbell

BINARY    = target/release/mbell
SERVICE   = mbell.service

.PHONY: all build check-deps install uninstall clean

all: build

check-deps:
	@command -v cargo >/dev/null 2>&1 || { echo "Error: cargo not found. Install Rust: https://rustup.rs"; exit 1; }
	@pkg-config --exists alsa 2>/dev/null || { echo "Error: ALSA dev headers not found. Install: sudo apt install libasound2-dev pkg-config"; exit 1; }

build: check-deps
	cargo build --release --locked

install: $(BINARY)
	install -Dm755 $(BINARY) $(DESTDIR)$(BINDIR)/mbell
	sed 's|/usr/bin/mbell|$(BINDIR)/mbell|g' $(SERVICE) | install -Dm644 /dev/stdin $(DESTDIR)$(SYSTEMDDIR)/mbell.service
	install -Dm644 LICENSE $(DESTDIR)$(LICENSEDIR)/LICENSE
	install -Dm644 README.md $(DESTDIR)$(DOCDIR)/README.md

uninstall:
	rm -f $(DESTDIR)$(BINDIR)/mbell
	rm -f $(DESTDIR)$(SYSTEMDDIR)/mbell.service
	rm -rf $(DESTDIR)$(LICENSEDIR)
	rm -rf $(DESTDIR)$(DOCDIR)

clean:
	cargo clean
