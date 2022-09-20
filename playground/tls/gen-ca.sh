#!/usr/bin/env bash
rm ca-key.pem
rm ca-cert.pem
rm ca-cert.srl
openssl req -x509 -newkey rsa:4096 -days 365 -keyout ca-key.pem -out ca-cert.pem -subj "/C=IT/O=MyCA/CN=MyCA"
