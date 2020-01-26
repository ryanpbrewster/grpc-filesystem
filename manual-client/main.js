const fs = require("fs");
const grpc = require("grpc");
const loader = require("@grpc/proto-loader");

async function main() {
  const definition = loader.loadSync("../proto/fs.proto");
  const proto = grpc.loadPackageDefinition(definition);
  const client = new Wrapper(new proto.fs.FileSystem("localhost:50051", grpc.credentials.createInsecure()));
  for (const dirname of ["/home", "/home/rpb", "/usr", "/usr/lib"]) {
    console.log(`> mkdir ${dirname}`);
    await client.mkdir(dirname);
    for (let i = 0; i < 3; i++) {
      const path = `${dirname}/foo-${i}.txt`;
      const content = `n = ${dirname.length + i}`;
      console.log(`> echo '${content}' > ${path}`);
      await client.write(path, Buffer.from(content));
    }
  }

  console.log(`> ls -R`);
  console.log(await client.lsrec("/"));

  console.log(`> exec <wasm>`);
  console.log(await client.exec(fs.readFileSync("example.wasm")));
}

class Wrapper {
  constructor(underlying) {
    this.underlying = underlying;
  }

  ls(path) {
    return new Promise((resolve, reject) => {
      this.underlying.list({ path }, (err, resp) => {
        if (err) {
          reject(err);
        } else {
          resolve(resp.paths);
        }
      });
    });
  }

  mkdir(path) {
    return new Promise((resolve, reject) => {
      this.underlying.mkdir({ path }, (err, resp) => {
        if (err) {
          reject(err);
        } else {
          resolve(resp);
        }
      });
    });
  }

  write(path, content) {
    return new Promise((resolve, reject) => {
      this.underlying.write({ path, content }, (err, resp) => {
        if (err) {
          reject(err);
        } else {
          resolve(resp);
        }
      });
    });
  }

  exec(wasm) {
    return new Promise((resolve, reject) => {
      this.underlying.exec({ wasm }, (err, resp) => {
        if (err) {
          reject(err);
        } else {
          resolve(resp);
        }
      });
    });
  }

  async lsrec(dirname) {
    const result = {};
    const children = await this.ls(dirname);
    for (const path of children) {
      result[path] = path.endsWith('/') ? await this.lsrec(`${dirname}${path}`) : null;
    }
    return result;
  }
}

main().catch(err => console.error(err));
