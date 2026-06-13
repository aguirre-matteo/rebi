pkgname=rebi-git
pkgver=0.1.0
pkgrel=1
pkgdesc="Rebi - A Keyboard and Mouse remapping tool (Frontend for keyd)"
arch=('x86_64')
url="https://github.com/aguirre-matteo/autokey"
license=('GPL2')
depends=('keyd' 'polkit' 'gtk3' 'libxkbcommon' 'libgl')
makedepends=('rust' 'cargo' 'pkg-config')
source=("git+$url.git")
md5sums=('SKIP')

build() {
  cd "$srcdir/autokey"
  cargo build --release --workspace
}

package() {
  cd "$srcdir/autokey"
  # Binaries
  install -Dm755 "target/release/rebi-gui" "$pkgdir/usr/bin/rebi-gui"
  install -Dm755 "target/release/rebi-helper" "$pkgdir/usr/bin/rebi-helper"
  
  # Polkit
  install -Dm644 "polkit/org.rebi.policy" "$pkgdir/usr/share/polkit-1/actions/org.rebi.policy"
  install -Dm644 "polkit/org.rebi.rules" "$pkgdir/usr/share/polkit-1/rules.d/org.rebi.rules"

  # Desktop Entry & Icons
  install -Dm644 "rebi.desktop" "$pkgdir/usr/share/applications/rebi.desktop"
  install -Dm644 "assets/logo-dark.png" "$pkgdir/usr/share/icons/hicolor/scalable/apps/rebi.png"
  install -Dm644 "assets/logo-light.png" "$pkgdir/usr/share/icons/hicolor/scalable/apps/rebi-light.png"
}
