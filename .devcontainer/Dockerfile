FROM rust:1.76-slim-bookworm

RUN apt-get update && \
    apt-get install -y cmake && \
    echo "alias grit='/workspaces/grit/target/debug/grit'" >> ~/.bashrc