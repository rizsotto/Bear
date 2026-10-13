FROM fedora:44

ENV RUSTUP_HOME=/root/.rustup \
    CARGO_HOME=/root/.cargo \
    PATH=/root/.cargo/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin

# time/bc/file/which: small diagnostics used by agent-driven measurement
# and inspection sessions (GNU time and bc are not in the base image).
#
# mdbook is pinned (--locked) to build the docs site under site/. Keep this
# version identical to the one the Pages CI workflow installs.
#
# beads (bd) is pinned to a release tarball, checked against its sha256.
# It tracks local development work; GitHub issues stay for user reports.
# BD_DISABLE_METRICS keeps its usage metrics (on by default) switched off.
ENV BD_DISABLE_METRICS=1
ARG BEADS_VERSION=1.3.1
ARG BEADS_SHA256=3219443a9734b89b93fb16ee8d65844759fa1b3cd3cf139c606b7353cfb0715c
RUN dnf -y install curl ca-certificates git gh pandoc time bc file which \
    && dnf -y group install c-development \
    && dnf -y clean all \
    && rm -rf /var/cache/dnf \
    && curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --profile default \
    && curl -fsSL https://claude.ai/install.sh | sh \
    && cargo install mdbook --version 0.5.4 --locked \
    && curl -fsSLo /tmp/beads.tar.gz https://github.com/gastownhall/beads/releases/download/v${BEADS_VERSION}/beads_${BEADS_VERSION}_linux_amd64.tar.gz \
    && echo "${BEADS_SHA256}  /tmp/beads.tar.gz" | sha256sum -c - \
    && tar -xzf /tmp/beads.tar.gz -C /usr/local/bin bd \
    && rm /tmp/beads.tar.gz
