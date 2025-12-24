import json

json_file = "data/HXMT-HE/2017/06/20170622_signals.json"

with open(json_file, "r") as f:
    data = json.load(f)

print(data[0]["events"][0])
