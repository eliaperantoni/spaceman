#!/usr/bin/env bash
rm server-cert.pem
openssl x509 -req -in server-req.pem -CA ca-cert.pem -days 365 -CAkey ca-key.pem -CAcreateserial -out server-cert.pem -extfile ext.cnf
