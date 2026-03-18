import TipMinimalApp from "../pages/tip-minimal/TipMinimalApp.svelte";
import { mount } from "svelte";

const app = mount(TipMinimalApp, { target: document.getElementById("app")! });

export default app;
