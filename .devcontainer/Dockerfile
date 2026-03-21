FROM fedora:43

ENV RUSTUP_HOME=/root/.rustup \
    CARGO_HOME=/root/.cargo \
    PATH=/root/.cargo/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin

RUN dnf -y install curl ca-certificates git gh pandoc nodejs-npm \
    && dnf -y group install c-development \
    && curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal \
    && rustup component add rustfmt \
    && rustup component add clippy-preview \
    && npm install -g @anthropic-ai/claude-code \
    && dnf -y clean all \
    && rm -rf /var/cache/dnf
