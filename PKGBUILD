# Maintainer: Stoa Ops <stoa-ops@github.com>
pkgname=mbell
pkgver=0.1.0
pkgrel=1
pkgdesc="A mindfulness bell daemon for Linux"
arch=('x86_64')
url="https://github.com/stoa-ops/mbell"
license=('MIT')
depends=('alsa-lib')
makedepends=('rust' 'cargo')
optdepends=(
    'pipewire-pulse: PipeWire audio support'
    'pulseaudio: PulseAudio audio support'
)

build() {
    cd "$startdir"
    cargo build --release --locked
}

package() {
    cd "$startdir"

    # Install binary
    install -Dm755 "target/release/mbell" "$pkgdir/usr/bin/mbell"

    # Install systemd user service
    install -Dm644 "mbell.service" "$pkgdir/usr/lib/systemd/user/mbell.service"

    # Install license
    install -Dm644 "LICENSE" "$pkgdir/usr/share/licenses/$pkgname/LICENSE"

    # Install documentation
    install -Dm644 "README.md" "$pkgdir/usr/share/doc/$pkgname/README.md"
}
