FROM ubuntu:19.10
RUN apt-get update && apt-get install -y python3-pip && pip3 install websockets
COPY ws.py .
COPY texlab .
COPY wrapper.sh .
CMD ./wrapper.sh