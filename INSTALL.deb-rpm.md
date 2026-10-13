# Установка на deb/rpm-дистрибутивах без сборки из сырцов вручную

Шаблон: см. `wmde-start-menu`, `wmde-panel`, `wmde-bg`, `wmde-randr`. Автор собирает пакеты у себя локально, как сейчас `makepkg -si`.

Опорные платформы — Debian stable и Fedora: `.deb` собираем на Debian (встаёт и на Ubuntu/Mint),
`.rpm` — на Fedora (встаёт и на RHEL/openSUSE). Наоборот не гарантируется.

Тулчейн берётся из `rust-toolchain.toml` (сейчас 1.93.0) — руками ставить версию не надо.

## Исходники рядом

Обычная сборка требует соседние чек-ауты (path-зависимости Cargo, см. `PKGBUILD` + `cargo metadata`):

```
work/
  wmde-osd/                    <- этот репозиторий (ветка wmde)
  libcosmic/                   <- git clone -b wmde https://github.com/Lin-WMDE/libcosmic.git
  wmde-comp/                   <- git clone -b wmde https://github.com/Lin-WMDE/wmde-comp.git
  wmde-settings-daemon/        <- git clone -b wmde https://github.com/Lin-WMDE/wmde-settings-daemon.git
  wmde-settings/               <- git clone -b wmde https://github.com/Lin-WMDE/wmde-settings.git
  wmde-randr/                  <- git clone -b wmde https://github.com/Lin-WMDE/wmde-randr.git
  wmde-dbus-settings-bindings/ <- git clone -b wmde https://github.com/Lin-WMDE/wmde-dbus-settings-bindings.git
```

Внутри `libcosmic` лежат git-сабмодули (`iced` ветки `wmde`, `cosmic-icons`).
Официальный URL сабмодуля `iced` — SSH (`git@github.com:`), без ключей не клонируется.
Обход без ключей:

```sh
git config --global url."https://github.com/".insteadOf "git@github.com:"
git -C libcosmic submodule update --init
```

## Собрать

```sh
# зависимости сборки: rustc (из rust-toolchain.toml) cargo just clang lld pkgconf polkit udev libinput + nfpm
cargo install --locked nfpm  # или бинарник с github.com/goreleaser/nfpm/releases

just deb   # -> target/wmde-osd_*.deb
just rpm   # -> target/wmde-osd-*.rpm
```

Версия — `1.10.0.<commits>.g<sha>`, как `pkgver()` в `PKGBUILD`.
По умолчанию сборка под чистый `x86-64`. Опционально под `x86-64-v3` (как в Arch-репозитории):

```sh
WMDE_CPU=x86-64-v3 just deb
```

## Офлайн-сборка (без сети к git-зависимостям)

`vendor.tar` в git не лежит (см. `.gitignore`), генерим на машине с сетью один раз
(соседние чек-ауты для резолва нужны):

```sh
just vendor          # -> vendor.tar (реестр + git-крейты + .cargo)
just deb-vendored    # сборка --frozen --offline -> target/*.deb
just rpm-vendored    # аналогично -> target/*.rpm
```

На машину без сети привозим `vendor.tar` файлом. Соседние чек-ауты тоже нужны:
path-зависимости вендором не покрываются, Cargo читает их с диска.

## Поставить

```sh
# Debian/Ubuntu/Mint
sudo apt install ./target/wmde-osd_*.deb

# Fedora/RHEL/openSUSE
sudo dnf install ./target/wmde-osd-*.rpm
```

Рантайм-зависимости подтянутся сами (см. `packaging/nfpm.yaml`).
`wmde-randr` идёт в recommends: без него работает всё, кроме идентификации дисплеев;
`apt`/`dnf` ставят Recommends по умолчанию, так что в сборе приедет сам.
Для звука из OSD нужен `wmde-settings` (опционально, см. PKGBUILD).
