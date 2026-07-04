FROM fedora:44

ENV RUSTUP_HOME=/root/.rustup \
    CARGO_HOME=/root/.cargo \
    PATH=/root/.cargo/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin

# time/bc/file/which: small diagnostics used by agent-driven measurement
# and inspection sessions (GNU time and bc are not in the base image).
RUN dnf -y install curl ca-certificates git gh pandoc time bc file which \
    && dnf -y group install c-development \
    && dnf -y clean all \
    && rm -rf /var/cache/dnf \
    && curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --profile default \
    && curl -fsSL https://claude.ai/install.sh | sh
