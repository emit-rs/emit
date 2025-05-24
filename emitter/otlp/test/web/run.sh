#!/bin/bash

wasm-pack build --target web --profiling
docker run -it -v "$(pwd):/usr/share/nginx/html:ro" -p 8080:80 nginx
