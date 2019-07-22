const grpc = require("grpc");
const loader = require("@grpc/proto-loader");

const definition = loader.loadSync("../proto/fs.proto");
const proto = grpc.loadPackageDefinition(definition);
const client = new proto.fs.FileSystem("localhost:50051", grpc.credentials.createInsecure());

async function main() {
  console.log("starting get...");
  await new Promise(resolve => {
    client.get({ path: "/foo/bar/baz.txt" }, (err, resp) => {
      if (err) {
        console.error(err);
      } else {
        console.log(resp.content.toString());
      }
      resolve();
    });
  });
}

main();
