#!/bin/bash
set -eu
out=@out@

# GNU make wrapper which invokes the make command twice, once to create the
# make database for processing by makedot, and then to actually run the make
# command.

_MAKEDB="$(mktemp)"
$out/libexec/make --just-print --print-data-base "$@" > "$_MAKEDB"
$out/libexec/makedot --targets --png \
  --rewrite _out \
  --rewrite TMPDIR \
  --rewrite FLOX_ENV \
  --rewrite BUILD_RESULT_FILE \
  $_MAKEDB
rm -f $_MAKEDB

exec $out/libexec/make "$@"
