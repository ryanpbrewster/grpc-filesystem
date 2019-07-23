const grpc = require("grpc");
const loader = require("@grpc/proto-loader");

const definition = loader.loadSync("../proto/fs.proto");
const proto = grpc.loadPackageDefinition(definition);
const client = new proto.fs.FileSystem("localhost:50051", grpc.credentials.createInsecure());

const asyncClient = {};
Object.values(client.$method_names).map(name => name.toLowerCase()).forEach(name => {
  asyncClient[name] = (input) => new Promise((resolve, reject) => {
    client[name](input, (err, resp) => {
      if (err) { reject(err); } else { resolve(resp); }
    });
  });
});

async function main() {
  for (const dirname of ["/home", "/home/rpb", "/usr", "/usr/lib"]) {
    console.log("mkdir " + dirname);
    await asyncClient.mkdir({ path: dirname });
    for (let i = 0; i < 10; i++) {
      await asyncClient.write({
        path: `${dirname}/foo-${i}.txt`,
        content: Buffer.from(`n = ${dirname.length + i}`),
      });
    }
  }

  await lsrec("/");
}

async function lsrec(dirname) {
  console.log("ls " + dirname);
  const list = await asyncClient.list({ path: dirname });
  console.log(list);
  for (const path of list.paths) {
    if (path.endsWith("/")) {
      await lsrec(dirname + path);
    }

  }
}

main().catch(err => console.error(err));
