# Update get_os function
get_os() {
    local os
    os=$(uname -s)
    case $os in
        Linux)
            echo "linux"
            ;;
        Darwin)
            echo "macos"
            ;;
        *)
            error "Unsupported operating system: $os"
            ;;
    esac
}

# Update get_arch function
get_arch() {
    local arch
    arch=$(uname -m)
    case $arch in
        x86_64)
            echo "x86_64"
            ;;
        arm64|aarch64)
            echo "aarch64"
            ;;
        *)
            error "Unsupported architecture: $arch"
            ;;
    esac
}

# Update asset name selection in install_binary()
# Replace the existing asset_name line with:
local asset_name
case "$os" in
    linux)
        asset_name="authguard-${os}-${arch}-musl.tar.gz"
        ;;
    macos)
        asset_name="authguard-${os}-${arch}.tar.gz"
        ;;
    *)
        error "Unsupported OS: $os"
        ;;
esac