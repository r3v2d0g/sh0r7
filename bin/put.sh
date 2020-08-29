#!/usr/bin/env bash
set -euo pipefail

VERSION="001"
PERMANENT="f"
APPEND_PATH="f"
FETCH="f"

DOMAIN=""
URL_PATH=""
REDIRECT=""

HELP() {
    echo "./put.sh [options] domain [path] redirect"
    echo
    echo "'path' must end with a forward slash ('/')"
    echo
    echo "if 'path' is not specified, 'redirect' becomes the default redirection for requests for"
    echo "URLs whose domain is 'domain' and that don't already match another entry"
    echo
    echo "options:"
    echo "-h, --help          show help"
    echo "-v, --version       show version"
    echo "-p, --permanent     make redirection permanent (HTTP code 302 instead of 307)"
    echo "-a, --append-path   append request path to the url"
    echo "-f, --fetch         fetch the url and serve its content (also fetches/stores the"
    echo "                    response from/to the cache; disables --permanent and --append-path)"
}

if [[ $# -eq 0 ]]; then
    HELP
fi

while [[ $# -gt 0 ]]; do
    case "$1" in
        -h|--help)
            HELP
            exit 0
            ;;
        -v|--version)
            echo "version $VERSION"
            exit 0
            ;;
        -p|--permanent)
            PERMANENT="t"
            shift
            ;;
        -a|--append-path)
            APPEND_PATH="t"
            shift
            ;;
        -f|--fetch)
            FETCH="t"
            shift
            ;;
        *)
            if [[ -z "$DOMAIN" ]]; then
                DOMAIN="$1"
                shift
                continue
            fi

            if [[ -z "$REDIRECT" ]]; then
                REDIRECT="$1"
                shift
                continue
            fi

            if [[ -z "$URL_PATH" ]]; then
                URL_PATH="$REDIRECT"
                REDIRECT="$1"
                shift
                continue
            fi

            HELP
            exit 1
            ;;
    esac
done

if [[ -z "$DOMAIN" ]] || [[ -z "$REDIRECT" ]]; then
    echo "expected at least 2 arguments"
    exit 1
fi

if [[ ! -z "$URL_PATH" ]] && [[ "${URL_PATH: -1}" != "/" ]]; then
    echo "'path' doesn't end with a forward slash"
    exit 1
fi

if [[ ! -f "$(dirname $0)"/../wrangler.toml ]]; then
    echo "file 'wrangler.toml' is missing"
    exit 1
fi

NAMESPACE_ID="$(cat "$(dirname $0)"/../wrangler.toml | grep '^id = ".*"$' | grep -o '\w\{32\}')"
KEY="$DOMAIN$URL_PATH"
VALUE="$VERSION:$PERMANENT:$APPEND_PATH:$FETCH:$REDIRECT"

echo "$KEY"
echo "$VALUE"

TEMP=$(mktemp)
echo "[{\"key\": \"$KEY\", \"value\": \"$VALUE\"}]" > $TEMP

# FIXME: using kv:key put and passing $KEY and $VALUE directly, remove the forward slash at the end of $KEY
wrangler kv:bulk put --namespace-id "$NAMESPACE_ID" $TEMP
