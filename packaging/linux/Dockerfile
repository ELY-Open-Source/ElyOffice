# glibc 2.28: runs on Debian 11, Ubuntu 20.04 onward.
FROM almalinux:8@sha256:8b469a3a78515e8a18ea8fc727e6a3679e1d0c5ba6f58d5b30be3d0d9b86cffe

RUN dnf install -y dnf-plugins-core epel-release \
  && dnf config-manager --set-enabled powertools \
  && dnf install -y \
    gcc-toolset-15-gcc gcc-toolset-15-gcc-c++ gcc-toolset-15-binutils \
    autoconf automake libtool make bison flex gperf nasm patch perl perl-IPC-Cmd perl-Time-Piece perl-Digest-SHA \
    python3.11 zip unzip which xz bzip2 libxslt findutils diffutils \
    curl ca-certificates git rsync patchelf file jq cmake ninja-build pkgconf-pkg-config \
    libcurl-devel expat-devel zlib-devel krb5-devel dbus-devel \
    libxkbcommon-devel libxkbcommon-x11-devel wayland-devel libX11-devel libxcb-devel fontconfig-devel \
  && alternatives --set python3 /usr/bin/python3.11 \
  && dnf clean all
# New C++ links in statically; old libstdc++ serves.
ENV PATH=/opt/rh/gcc-toolset-15/root/usr/bin:/opt/cargo/bin:$PATH RUSTUP_HOME=/opt/rustup
# As rust-toolchain.toml pins it.
RUN case $(uname -m) in \
      x86_64) sum=5be191e8c70a5007074254cb74d78dbfdd25cd5a61c27de7b8e5410122ec24f4 ;; \
      aarch64) sum=a9c168dbea81a84671c6b7a5c1795cb0eb699020110d107ddfca427bd797e697 ;; \
    esac \
  && curl -fsSL -o /tmp/rustup-init "https://static.rust-lang.org/rustup/archive/1.29.2/$(uname -m)-unknown-linux-gnu/rustup-init" \
  && echo "$sum  /tmp/rustup-init" | sha256sum -c - \
  && chmod +x /tmp/rustup-init \
  && CARGO_HOME=/opt/cargo /tmp/rustup-init -y --no-modify-path --profile minimal --default-toolchain 1.96.0 -c rustfmt,clippy \
  && rm /tmp/rustup-init \
  && chmod -R a+rX /opt/rustup /opt/cargo
