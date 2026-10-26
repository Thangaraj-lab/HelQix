# K7 render-stream spike image. CPU-only by default (x264 software encode, SwiftShader WebGL).
# On a GPU host run with --gpus all and an image that also has the NVIDIA GStreamer codecs (see README).
#   docker build -t helqix-stream streaming/
#   docker run --rm --cap-add NET_ADMIN --shm-size 2g -v "$PWD/out:/out" helqix-stream python3 /app/measure.py --out /out
FROM ubuntu:24.04

ENV DEBIAN_FRONTEND=noninteractive PYTHONUNBUFFERED=1
RUN apt-get update && apt-get install -y --no-install-recommends \
      python3 python3-venv python3-gi gir1.2-gstreamer-1.0 gir1.2-gst-plugins-base-1.0 gir1.2-gst-plugins-bad-1.0 \
      gstreamer1.0-tools gstreamer1.0-plugins-base gstreamer1.0-plugins-good gstreamer1.0-plugins-bad gstreamer1.0-plugins-ugly \
      gstreamer1.0-nice gstreamer1.0-x xvfb iproute2 coturn nodejs npm ca-certificates curl \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY requirements.txt /app/requirements.txt
RUN python3 -m venv --system-site-packages /venv && /venv/bin/pip install --no-cache-dir -r /app/requirements.txt \
    && /venv/bin/python -m playwright install --with-deps chromium chrome
ENV PATH=/venv/bin:$PATH
COPY web/package.json web/package-lock.json /app/web/
RUN cd /app/web && npm ci --no-audit --no-fund
COPY server /app/server
COPY web /app/web
COPY measure.py /app/measure.py
EXPOSE 8080
CMD ["python3", "/app/server/stream_server.py"]
