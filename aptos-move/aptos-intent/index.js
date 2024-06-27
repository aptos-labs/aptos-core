const rust = import("./pkg");

rust
  .then((m) => {
    return m.get_module("0x1", "account").then((data) => {
      console.log(data);
    });
  })
  .catch(console.error);
