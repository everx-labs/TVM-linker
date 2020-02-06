const path = require('path');
const fs = require('fs');

if(process.argv.length < 5) {
    console.log(`Usage: node ${path.relative(process.cwd(), __filename)} <file> <from string> <to string>`);
    process.exit(1);
}

const file = path.join(process.cwd(), process.argv[2]);
const fromStr = process.argv[3];
const toStr = process.argv[4];

if(!fs.existsSync(file)) {
    console.log(`File ${file} not found`);
    process.exit(1);
}

let cont = fs.readFileSync(file);
const re = new RegExp(fromStr,'g');
cont = cont.toString().replace(re,toStr);
fs.writeFileSync(file, cont);


