import asyncio
import time
from bleak import BleakClient
from bleak.exc import BleakDBusError

address = "86:FC:E6:00:BF:C1"
uuid = "937312e0-2354-11eb-9f10-fbc30a62cf38"
report_uuid = "957312e0-2354-11eb-9f10-fbc30a62cf38"
data = b'a' * 199

async def write(client: BleakClient, msg_len: int, response: bool):
    try:
        msg = b'a' * msg_len
        for i in range(50):
            #start = time.time()
            await client.write_gatt_char(uuid, msg, response=response)
            #end = time.time()
            #print(end - start)
        await client.read_gatt_char(report_uuid)
    except BleakDBusError:
        print("failed, maybe a write command with too high MTU?")
    

async def main(address):
    async with BleakClient(address) as client:
        for resp in [True, False]:
            for msg_len in [20, 100, 200]:
                print("benching:", resp, msg_len)
                await write(client, msg_len, resp)


asyncio.run(main(address))
