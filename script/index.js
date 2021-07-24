const CLASS_TYPE_CODE_HASH = "0x095b8c0b4e51a45f953acd1fcd1e39489f2675b4bc94e7af27bb38958790e3fc";
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
