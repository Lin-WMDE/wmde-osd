# Maintainer: WMDE <https://wmde.fun>
# Contributor: System76 <info@system76.com> (original cosmic-osd)
#
# Builds our fork Lin-WMDE/wmde-osd (branch wmde). Kept WMDE component (minimal
# edition): the volume/brightness/power OSD + polkit agent. Installs alongside
# cosmic-osd (own binary wmde-osd + own D-Bus name fun.wmde.Osd), so NO
# conflicts/replaces cosmic-osd and NO compat symlinks.
pkgname=wmde-osd
pkgver=0.1.0
pkgrel=1
pkgdesc="WMDE on-screen display and polkit agent (fork of cosmic-osd) - owns fun.wmde.Osd"
arch=('x86_64')
url="https://wmde.fun"
license=('GPL-3.0-or-later')
# runtime: wayland client (libcosmic winit/wayland), polkit agent helper at
# /usr/libexec/polkit-agent-helper-1, and cosmic-randr for display identify.
# Verify with namcap after first build.
depends=('glibc' 'gcc-libs' 'wayland' 'polkit' 'wmde-randr')
optdepends=('wmde-settings: Sound Settings shortcut from the volume OSD')
makedepends=('rust' 'cargo' 'just' 'git' 'wayland' 'clang' 'lld' 'pkgconf' 'polkit')
source=("$pkgname::git+https://github.com/Lin-WMDE/wmde-osd.git#branch=wmde")
sha256sums=('SKIP')

pkgver() {
  cd "$srcdir/$pkgname"
  # WMDE unified version: 1.5 (libcosmic base) . <commits since nearest tag> . g<short>.
  local desc
  desc=$(git describe --long --tags --abbrev=7 2>/dev/null || true)
  if [ -n "$desc" ]; then
    printf '1.5.%s.g%s' "$(printf '%s' "$desc" | sed -E 's/.*-([0-9]+)-g[0-9a-f]+$/\1/')" "$(git rev-parse --short=7 HEAD)"
  else
    printf '1.5.%s.g%s' "$(git rev-list --count HEAD)" "$(git rev-parse --short=7 HEAD)"
  fi
}

build() {
  cd "$srcdir/$pkgname"
  # x86-64-v3 (AVX2/BMI2) baseline for the WMDE repo; runs on Haswell+ (and the VM).
  export RUSTFLAGS="${RUSTFLAGS:+$RUSTFLAGS }-C target-cpu=x86-64-v3"
  just build-release
}

package() {
  cd "$srcdir/$pkgname"
  # installs /usr/bin/wmde-osd (name from justfile)
  just rootdir="$pkgdir" prefix=/usr install
  install -Dm644 LICENSE "$pkgdir/usr/share/licenses/$pkgname/LICENSE"
}
