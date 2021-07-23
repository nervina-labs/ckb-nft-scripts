const CLASS_TYPE_CODE_HASH = "0xfaeecc12e2f4183fe9fe12d1d88e75b8d12a2ca4b117a490778da3cce8dee287";
function hexToBytes(hex) {
  if (!hex) {
    return new Uint8Array();
  }
  let temp = hex.startsWith("0x") ? hex.slice(2) : hex;

  let bytes = [];
  const len = temp.length;
  for (let i = 0; i < len; i += 2) {
    bytes.push(parseInt(temp.substr(i, 2), 16));
  }

  return new Uint8Array(bytes);
}

console.log(hexToBytes(CLASS_TYPE_CODE_HASH));
