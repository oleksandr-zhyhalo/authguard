# authguard 🛡️

<div align="center">

![GitHub release (latest by date)](https://img.shields.io/github/v/release/oleksandr-zhyhalo/authguard)
![Rust Version](https://img.shields.io/badge/rust-1.70%2B-blue.svg)

A secure credentials manager for AWS IoT devices with local caching and multi-environments setup.

[Installation](#Installation) •
[Features](#features) •
[Usage](#usage) •
[Configuration](#configuration) •
[Contributing](#contributing)

</div>

## ✨ Features

- 🔐 **Secure mTLS Authentication**: Uses device certificates for AWS IoT authentication
- 🔄 **Automatic Credential Management**: Handles AWS credential rotation
- 📦 **AWS CLI Integration**: Works seamlessly with AWS CLI credential_process
- 📝 **Structured Logging**: JSON logging for audit trails
- 🔍 **Detailed Error Handling**: Clear error messages and proper error propagation
- 💻 **Cross-Platform**: Statically linked binary works on any Linux system
- 💾 **Credential Caching**: Local caching mechanism for AWS credentials to reduce unnecessary network calls

## 🚀 Installation

### Using Install Script (Recommended)

```bash
curl -o- https://raw.githubusercontent.com/oleksandr-zhyhalo/authguard/main/install.sh | sudo bash
# or with wget
wget -qO- https://raw.githubusercontent.com/oleksandr-zhyhalo/authguard/main/install.sh | sudo bash
```

### Manual Installation

1. Download the latest release for your platform from [releases page](https://github.com/oleksandr-zhyhalo/authguard/releases)
2. Install manually:
```bash
# Extract the archive
tar xzf authguard-linux-*.tar.gz

# Create directories
sudo mkdir -p /etc/authguard /var/log/authguard

# Install binary
sudo install -m 755 authguard/authguard /usr/local/bin/

# Set up config and logs (replace 'your-username' with your actual username)
sudo chown your-username:your-username /etc/authguard /var/log/authguard
sudo chmod 700 /etc/authguard /var/log/authguard

# Install config if needed
sudo install -m 600 -o your-username authguard/authguard.toml.sample /etc/authguard/authguard.conf
```

## 📚 Configuration

### AWS IoT Setup

1. Create an IoT thing and download certificates
2. Create a role alias in AWS IoT
3. Attach appropriate policies to your certificates

For more details read:
[Authorizing direct calls to AWS services using AWS IoT Core credential provider
   ](https://docs.aws.amazon.com/iot/latest/developerguide/authorizing-direct-aws.html)

### Configuration File

Create or edit `/etc/authguard/authguard.toml`:
```toml
cache_dir = "/var/cache/authguard"
log_dir = "/var/log/authguard"
circuit_breaker_threshold = 5
cool_down_seconds = 120

[environment]
current = "dev"

[environment.dev]
aws_iot_endpoint = "dev-ats.iot.us-west-2.amazonaws.com"
role_alias = "dev-role-alias"
cert_path = "/etc/authguard/dev/cert.pem"
key_path = "/etc/authguard/dev/key.pem"
ca_path = "/etc/authguard/dev/root-ca.pem"

[environment.prod]
aws_iot_endpoint = "prod-ats.iot.us-west-2.amazonaws.com"
role_alias = "prod-role-alias"
cert_path = "/etc/authguard/prod/cert.pem"
key_path = "/etc/authguard/prod/key.pem"
ca_path = "/etc/authguard/prod/root-ca.pem"
```

### AWS CLI Integration

Add to your AWS CLI config (`~/.aws/config`):
```ini
[profile your-profile]
credential_process = /usr/local/bin/authguard
```

## 📂 Directory Structure

```
/etc/authguard/
├── authguard.toml         # Main configuration
└── authguard.toml.sample  # Sample configuration

/var/log/authguard/
├── authguard.log         # Current application log
└── authguard.log.*      # Rotated log files

/var/cache/authguard/
├── creds_cache.json     # Cached credentials
└── cb_state.json       # Circuit breaker state

/usr/local/bin/
└── authguard            # Binary executable
```

## 🔨 Usage

### Testing Configuration

Verify your setup:
```bash
# Test credential retrieval
aws sts get-caller-identity --profile your-profile

# Check logs
tail -f /var/log/authguard/authguard.log
```

### Common Operations

```bash
# Direct credential retrieval
authguard

# With debug output
AWS_PROFILE=your-profile aws sts get-caller-identity --debug
```

## 📄 License

See the [LICENSE](LICENSE) file for details.

---

<div align="center">
Made with ❤️ for secure AWS IoT authentication

[Report Bug](https://github.com/oleksandr-zhyhalo/authguard/issues) • [Request Feature](https://github.com/oleksandr-zhyhalo/authguard/issues)
</div>
