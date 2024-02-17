# Worterbuch Dockerfile for x86_64
#
# Copyright (C) 2024 Michael Bachmann
#
# This program is free software: you can redistribute it and/or modify
# it under the terms of the GNU Affero General Public License as published by
# the Free Software Foundation, either version 3 of the License, or
# (at your option) any later version.
#
# This program is distributed in the hope that it will be useful,
# but WITHOUT ANY WARRANTY; without even the implied warranty of
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
# GNU Affero General Public License for more details.
#
# You should have received a copy of the GNU Affero General Public License
# along with this program.  If not, see <https://www.gnu.org/licenses/>.

FROM messense/rust-musl-cross:x86_64-musl AS worterbuch-builder
WORKDIR /src/worterbuch
COPY . .
RUN cargo build -p worterbuch --release

FROM scratch
WORKDIR /app
COPY --from=worterbuch-builder /src/worterbuch/target/x86_64-unknown-linux-musl/release/worterbuch .
ENV RUST_LOG=info
ENV WORTERBUCH_WS_BIND_ADDRESS=0.0.0.0
ENV WORTERBUCH_TCP_BIND_ADDRESS=0.0.0.0
ENV WORTERBUCH_USE_PERSISTENCE=true
ENV WORTERBUCH_DATA_DIR=/data
ENV WORTERBUCH_PERSISTENCE_INTERVAL=5
ENV WORTERBUCH_WS_SERVER_PORT=80
ENV WORTERBUCH_TCP_SERVER_PORT=9090
ENV WORTERBUCH_SINGLE_THREADED=false
ENV WORTERBUCH_WEBAPP=false
VOLUME [ "/data" ]
CMD ["./worterbuch"]
