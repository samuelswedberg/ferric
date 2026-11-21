import { mount } from "svelte";
import App from "./App.svelte";

const target = document.getElementById("app");

if (!target) {
  throw new Error("Ferric UI mount point was not found");
}

mount(App, { target });
