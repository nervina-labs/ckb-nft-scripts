const CLASS_TYPE_CODE_HASH = "0xd51e6eaf48124c601f41abe173f1da550b4cbca9c6a166781906a287abbb3d9a";
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
