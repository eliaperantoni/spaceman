#!/usr/bin/env bash
rm server-key.pem
rm server-req.pem
openssl req -newkey rsa:4096 -nodes -keyout server-key.pem -out server-req.pem -subj "/C=IT/O=MyServer/CN=MyServer"
