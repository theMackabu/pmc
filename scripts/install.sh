#!/bin/bash

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
BLUE='\033[0;34m'
PURPLE='\033[0;35m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color
GRAY='\033[1;30m'
BLUEBIRD='\033[38;5;39m'

VERSION="1.8.0"
RELEASE_URL="https://github.com/theMackabu/pmc/releases/download/v$VERSION"
INSTALL_PATH="/usr/bin/pmc"
BIN_NAME="pmc"
ARCH=$(uname -m)
BIN_URL="$RELEASE_URL/pmc_${VERSION}_linux_${ARCH}.zip"
TMP_DIR=$(mktemp -d)

#if [ "$EUID" -ne 0 ]; then
#  echo -e "${RED}💥 Permission denied. Please run as root or use sudo.${NC}"
#  exit 1
#fi

case $ARCH in
  amd64)
    ;;
  aarch64)
    ;;
  *)
    echo -e "${RED}💥 Unsupported architecture: $ARCH${NC}"
    exit 1
    ;;
esac

spin() {
  local -a marks=( '/' '-' '\' '|' )
  while true; do
    for mark in "${marks[@]}"; do
      printf "\r${YELLOW}[%s]${NC}" "$mark"
      sleep 0.1
    done
  done
}

download() {
  echo -ne "${BLUE}✨ Downloading... ${NC}"
  curl -L $BIN_URL -o "$TMP_DIR/$BIN_NAME.zip" &> /dev/null &
  local curl_pid=$!
  spin &
  local spin_pid=$!

  wait $curl_pid
  kill $spin_pid
  printf "\r"

  if [ $? -ne 0 ]; then
    echo -e "${RED}💥 Failed to download!${NC}"
    exit 1
  fi
  echo -e "${GREEN}🪄 Download completed!${NC}"
}

unzip_file() {
  echo -ne "${CYAN}📦 Unzipping... ${NC}"
  unzip -o "$TMP_DIR/$BIN_NAME.zip" -d $TMP_DIR &> /dev/null &
  local unzip_pid=$!
  spin &
  local spin_pid=$!

  wait $unzip_pid
  kill $spin_pid
  printf "\r"

  if [ $? -ne 0 ]; then
    echo -e "${RED}💥 Failed to unzip!${NC}"
    exit 1
  fi
  echo -e "${GREEN}🪄 Unzipping completed!${NC}"
}

install() {
  echo -ne "${PURPLE}✨ Installing... ${NC}"
  sudo mv "$TMP_DIR/$BIN_NAME" $INSTALL_PATH &> /dev/null &
  local mv_pid=$!
  spin &
  local spin_pid=$!

  wait $mv_pid
  kill $spin_pid
  printf "\r"

  if [ $? -ne 0 ]; then
    echo -e "${RED}💥 Failed to install!${NC}"
    exit 1
  fi
  echo -e "${GREEN}🪄 Installation completed!${NC}"
}

# entrypoint
echo -e "${BLUEBIRD}
██████╗ ███╗   ███╗ ██████╗
██╔══██╗████╗ ████║██╔════╝
██████╔╝██╔████╔██║██║
██╔═══╝ ██║╚██╔╝██║██║
██║     ██║ ╚═╝ ██║╚██████╗
╚═╝     ╚═╝     ╚═╝ ╚═════╝
${GRAY}A simple and easy to use PM2 alternative${NC}"
echo -e "${GRAY}Version: ${BLUEBIRD}${VERSION}${NC}"
echo

download
unzip_file
install

chmod +x $INSTALL_PATH