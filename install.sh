#!/bin/bash
set -euo pipefail

##########################
# Configuration Settings #
##########################

# Adjust these variables to match your GitHub repo and release asset names.
# Example:
#   - GITHUB_OWNER: Your GitHub username or organization.
#   - GITHUB_REPO: The repository name.
#   - RELEASE_TAG: The release version (e.g., "v1.0.0").
#   - ASSET_NAME: The name of the tarball that contains the binary and sample config.
GITHUB_OWNER="oleksandr-zhyhalo"
GITHUB_REPO="authguard"
RELEASE_TAG="v1.0.0"
ASSET_NAME="authguard-linux-amd64.tar.gz"

# Directories and file locations
INSTALL_BIN="/usr/local/bin"       # Where the binary will be installed.
CONFIG_DIR="/etc/authguard"        # Configuration directory.
LOG_DIR="/var/log/authguard"       # Log directory.
SAMPLE_CONFIG="authguard.conf.sample"  # Name of the sample configuration file in the asset.
BINARY_NAME="authguard"                # Name of the binary executable.

##########################
# Pre-Installation Checks#
##########################

# Ensure the script is run as root to allow installation in system directories.
if [[ $EUID -ne 0 ]]; then
    echo "This script must be run as root. Try: sudo $0"
    exit 1
fi

##########################
# Download the Release   #
##########################

echo "Downloading $BINARY_NAME from GitHub release..."

# Construct the download URL. GitHub release assets are typically available at:
# https://github.com/<owner>/<repo>/releases/download/<tag>/<asset_name>
DOWNLOAD_URL="https://github.com/${GITHUB_OWNER}/${GITHUB_REPO}/releases/download/${RELEASE_TAG}/${ASSET_NAME}"

# Create a temporary working directory
TMP_DIR=$(mktemp -d)
echo "Using temporary directory: $TMP_DIR"

# Download the asset using curl
curl -L -o "${TMP_DIR}/${ASSET_NAME}" "${DOWNLOAD_URL}"

##########################
# Extract the Archive    #
##########################

echo "Extracting archive..."
tar -xzf "${TMP_DIR}/${ASSET_NAME}" -C "${TMP_DIR}"

##########################
# Install the Binary     #
##########################

echo "Installing binary to ${INSTALL_BIN}..."

# Assume the archive contains the binary with the expected name.
if [[ ! -f "${TMP_DIR}/${BINARY_NAME}" ]]; then
    echo "Error: Binary ${BINARY_NAME} not found in the archive."
    exit 1
fi

# Copy the binary to the installation directory and set executable permissions.
cp "${TMP_DIR}/${BINARY_NAME}" "${INSTALL_BIN}/"
chmod 755 "${INSTALL_BIN}/${BINARY_NAME}"

##########################
# Setup Configuration    #
##########################

echo "Setting up configuration directory at ${CONFIG_DIR}..."

# Create the configuration directory if it doesn't exist.
mkdir -p "${CONFIG_DIR}"

# Copy the sample configuration file if it exists.
if [[ -f "${TMP_DIR}/${SAMPLE_CONFIG}" ]]; then
    cp "${TMP_DIR}/${SAMPLE_CONFIG}" "${CONFIG_DIR}/authguard.conf"
    # Set permissions to allow read access (owner: root, group: root).
    chmod 644 "${CONFIG_DIR}/authguard.conf"
    echo "Sample configuration installed to ${CONFIG_DIR}/authguard.conf"
else
    echo "Warning: Sample configuration file ${SAMPLE_CONFIG} not found in the archive."
fi

##########################
# Setup Log Directory    #
##########################

echo "Setting up log directory at ${LOG_DIR}..."

# Create the log directory with the proper permissions.
mkdir -p "${LOG_DIR}"
chmod 750 "${LOG_DIR}"

##########################
# Cleanup and Finish     #
##########################

echo "Cleaning up temporary files..."
rm -rf "${TMP_DIR}"

echo "Installation complete!"
echo "You can now run '${BINARY_NAME}' from ${INSTALL_BIN}."
