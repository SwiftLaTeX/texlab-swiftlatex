import asyncio
import websockets
import json
import re


def rewrite_server2client_path(content, project_id):
    content = content.replace('minio/%s/' % project_id, '')
    return content

def rewrite_client2server_path(content, project_id):
    regex = r"file:\/\/\/([-_A-Za-z0-9 .\/]*)"
    subst = "file:///minio/%s/\\g<1>"
    return re.sub(regex, subst % project_id, content)


def obtain_project_id(path):
    if path.startswith('/u/') and len(path) >= 20:
        to_valid_path = path[3:19]
        for i in range(16):
            if ('0' <= to_valid_path[i] <= '9') or ('a' <= to_valid_path[i] <= 'z') or ('A' <= to_valid_path[i] <= 'Z'):
                continue
            return None
        return to_valid_path
    return None


async def echo(websocket, path):
    print('path %s' % path)
    project_id = obtain_project_id(path)
    if project_id is None:
        return
    print("Start serving with id %s" % project_id)

    try:
        handshake = await websocket.recv()
        packet = json.loads(handshake)
        if packet["kind"] != "open":
            print("Bad handshake with id %s" % project_id)
            return

        channel_id = packet["id"]
        resp = {
            "kind": "ready",
            "id": channel_id
        }
        await websocket.send(json.dumps(resp))
    except:
        print("Handshake failed %s" % project_id)
        return

    try:
        remote_reader, remote_writer = await asyncio.open_connection(
            host="localhost", port="9998")
    except:
        print("Texlab not ready %s" % project_id)
        return

    task_1 = asyncio.create_task(server_handler(websocket, project_id, channel_id, remote_reader))
    task_2 = asyncio.create_task(client_handler(websocket, project_id, channel_id, remote_writer))
    try:
        await asyncio.wait({task_1, task_2}, return_when=asyncio.FIRST_COMPLETED)
    except Exception as e:
        print('Accident detected: %s' % e)
    finally:
        print('Bye %s' % project_id)
        remote_writer.close()


async def client_handler(websocket, project_id, channel_id, remote_writer):
    async for message in websocket:
        packet = json.loads(message)
        if packet["kind"] == "data":
            content = packet["content"]
            content = rewrite_client2server_path(content, project_id)
            print(content)
            remote_writer.write(pack_remote_message(content))
        else:
            break


async def server_handler(websocket, project_id, channel_id, remote_reader):
    while not remote_reader.at_eof():
        payload = await read_remote_message(remote_reader)
        if payload is None:
            return
        payload = rewrite_server2client_path(payload.decode('utf-8'), project_id)
        data = {"kind": "data", "id": channel_id, "content": payload}
        print(data)
        await websocket.send(json.dumps(data))

    # traceback.print_exc()


async def read_remote_message(reader):
    headers = {}
    while True:
        data = await reader.readline()

        if not data:
            return None

        if data == b"\r\n":
            break

        name, _, value = data.decode().partition(":")
        headers[name.strip()] = value.strip()

    content_length = int(headers["Content-Length"])
    body = await reader.readexactly(content_length)
    return body


# Json String to Pack
def pack_remote_message(body):
    content_length = len(body.encode("utf-8"))
    response = f"Content-Length: {content_length}\r\n\r\n{body}"
    return response.encode("utf-8")



asyncio.get_event_loop().run_until_complete(
    websockets.serve(echo, '0.0.0.0', 8080))
print('Server started!')
asyncio.get_event_loop().run_forever()
