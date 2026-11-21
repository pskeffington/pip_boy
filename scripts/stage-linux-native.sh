#!/usr/bin/env bash
# Stage a verified, already built native Linux pipboy into an isolated rootfs.
# This never installs to the running host, starts a service, or enables autostart.
set -euo pipefail

die() { printf 'Rust-Boy Linux stage: %s\n' "$*" >&2; exit 1; }
[[ $# -eq 2 ]] || die 'usage: bash scripts/stage-linux-native.sh BUILT_PIPBOY ABSOLUTE_STAGING_ROOT'
binary="$1"
dest="\${2%/}"
root="$(cd "$(dirname "\${BASH_SOURCE[0]}")/.." && pwd)"
[[ -f "$binary" && -x "$binary" && ! -L "$binary" ]] || die 'native binary missing, non-executable, or a symlink'
[[ "$dest" == /* && "$dest" != / && -d "$dest" ]] || die 'requires an existing absolute staging root other than /'

# Reject symlink redirection anywhere in the supplied staging root.
probe="$dest"
while [[ "$probe" != / ]]; do
  [[ ! -L "$probe" ]] || die 'staging path contains a symlink'
  probe="$(dirname -- "$probe")"
done

paths=(
  usr/local/bin/pipboy
  usr/local/bin/rust-boy-launcher
  usr/lib/systemd/user/rust-boy-dashboard.service
  usr/share/applications/org.rustboy.RustBoy.desktop
  usr/share/licenses/rust-boy/LICENSE
  usr/share/rust-boy/native-binary.sha256
)
for path in "\${paths[@]}"; do
  cursor="$dest"
  IFS=/ read -ra components <<< "$path"
  for component in "\${components[@]}"; do
    cursor="$cursor/$component"
    [[ ! -L "$cursor" ]] || die "staging destination contains a symlink: $path"
  done
  [[ ! -e "$dest/$path" ]] || die "refusing overwrite of $path"
done

install -Dm0755 "$binary" "$dest/usr/local/bin/pipboy"
install -Dm0755 "$root/linux/rust-boy-launcher" "$dest/usr/local/bin/rust-boy-launcher"
install -Dm0644 "$root/linux/rust-boy-dashboard.service" "$dest/usr/lib/systemd/user/rust-boy-dashboard.service"
install -Dm0644 "$root/linux/org.rustboy.RustBoy.desktop" "$dest/usr/share/applications/org.rustboy.RustBoy.desktop"
install -Dm0644 "$root/LICENSE" "$dest/usr/share/licenses/rust-boy/LICENSE"
source_hash="$(sha256sum "$binary" | awk '{print $1}')"
staged_hash="$(sha256sum "$dest/usr/local/bin/pipboy" | awk '{print $1}')"
[[ "$source_hash" == "$staged_hash" ]] || die 'staged binary hash mismatch'
install -d -m0755 "$dest/usr/share/rust-boy"
printf '%s\n' "$staged_hash" > "$dest/usr/share/rust-boy/native-binary.sha256"
printf 'Staged Rust-Boy native Linux payload (%s); no host install or service enablement\n' "$staged_hash"
