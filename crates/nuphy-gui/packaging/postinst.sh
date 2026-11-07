#!/bin/sh
# Apply the bundled hidraw rule right away so the keyboard opens without a reboot.
if command -v udevadm >/dev/null 2>&1; then
  udevadm control --reload-rules || true
  udevadm trigger --subsystem-match=hidraw || true
fi
exit 0
