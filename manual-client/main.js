const grpc = require("grpc");
const loader = require("@grpc/proto-loader");

const definition = loader.loadSync("../proto/fs.proto");
const proto = grpc.loadPackageDefinition(definition);
const client = new proto.fs.FileSystem("localhost:50051", grpc.credentials.createInsecure());

async function main() {
  console.log("echo 'Hello, World!' > /foo.txt");
  await new Promise(resolve => {
    client.write({ path: "/foo.txt", content: Buffer.from("Hello, World!") }, (err, resp) => {
      if (err) {
        console.error(err);
      } else {
        console.log(resp);
      }
      resolve();
    });
  });

  console.log("ls /");
  await new Promise(resolve => {
    client.list({ path: "/" }, (err, resp) => {
      if (err) {
        console.error(err);
      } else {
        console.log(resp);
      }
      resolve();
    });
  });

  console.log("cat /foo.txt");
  await new Promise(resolve => {
    client.get({ path: "/foo.txt" }, (err, resp) => {
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
