#!/bin/sh

if [ "$#" -ne 1 ]; then
  echo "USAGE: $0 image" >&2
  exit 1
fi

if ! [ -e "$1" ]; then
  echo "ERROR: $1 not found" >&2
  exit 1
fi

IMAGE_PATH="$1"
IMAGE_BASENAME="$(basename ${IMAGE_PATH})"
PORTER_CMD="cargo run -q --release --"
IMAGE_TEMP_DIR="$1.temp"

mkdir ${IMAGE_TEMP_DIR}
for t in `seq 0 255`; do
    ${PORTER_CMD} $t ${IMAGE_PATH}
    mv "sorted-${IMAGE_BASENAME}" "${IMAGE_TEMP_DIR}"/$(printf '%03d' $t)-"${IMAGE_BASENAME}"
    printf "\r%s" $t;
done;
printf '\r\n'

ffmpeg -f image2 -framerate 25 -i "${IMAGE_TEMP_DIR}/%03d-${IMAGE_BASENAME}" -vcodec libx264 -crf 22 "${IMAGE_BASENAME}.mp4"
