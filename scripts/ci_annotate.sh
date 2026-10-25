#!/usr/bin/env bash
# Prints the tail of a log as one GitHub Actions error annotation, so failure details are readable
# through the public API without repo-admin log access.
# Usage: scripts/ci_annotate.sh <title> <file> [lines]   (never fails)
title="$1"; file="$2"; lines="${3:-30}"
[ -f "$file" ] || { echo "::error title=$title::(no file $file)"; exit 0; }
body=$(tail -n "$lines" "$file" | sed -e 's/%/%25/g' -e 's/\r//g' | awk 'BEGIN{ORS="%0A"} {print}')
echo "::error title=$title::$body"
exit 0
