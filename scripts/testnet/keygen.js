const c=require('crypto');const A='123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz';
function b58(b){let n=BigInt('0x'+Buffer.from(b).toString('hex')||'0');let s='';while(n>0n){s=A[Number(n%58n)]+s;n/=58n;}for(const x of b){if(x===0)s='1'+s;else break;}return s;}
const {publicKey,privateKey}=c.generateKeyPairSync('ed25519');
const pub=publicKey.export({format:'jwk'});const prv=privateKey.export({format:'jwk'});
const pk=Buffer.from(pub.x,'base64url');const sk=Buffer.concat([Buffer.from(prv.d,'base64url'),pk]);
console.log(JSON.stringify({public_key:'ed25519:'+b58(pk),private_key:'ed25519:'+b58(sk)}));
