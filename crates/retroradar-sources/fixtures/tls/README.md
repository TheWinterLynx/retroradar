# Test-only TLS material

Dedicated P-256 test CA and localhost leaf certificate, generated 2026-11-01.
The leaf has CA:FALSE, serverAuth EKU, digitalSignature usage and DNS:localhost SAN.
The root has CA:TRUE,pathlen:0. Both have a 100-year test validity.
`server-key.der` is a deliberately public, test-only unencrypted PKCS#8 key.
Never trust this CA in an OS/browser or use this key outside unit tests.
The CA private key is not included. No external certificate generator is needed
at build or test time. RustCrypto performs both sides of the in-memory handshake.
