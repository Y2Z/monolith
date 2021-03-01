FROM rust

WORKDIR /usr/local/src/
RUN curl -s https://api.github.com/repos/y2z/monolith/releases/latest \
    | grep "tarball_url.*\"," \
    | cut -d '"' -f 4 \
    | wget -qi - -O monolith.tar.gz

RUN tar xfz monolith.tar.gz \
    && mv Y2Z-monolith-* monolith \
    && rm monolith.tar.gz

WORKDIR /usr/local/src/monolith
RUN ls -a
RUN make install

WORKDIR /tmp
CMD ["/usr/local/cargo/bin/monolith"]
