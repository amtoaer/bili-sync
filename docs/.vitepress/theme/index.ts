import DefaultTheme from "vitepress/theme";
import { onMounted, watch, nextTick } from "vue";
import { useRoute } from "vitepress";
import mediumZoom from "medium-zoom";
import "./index.css";

export default {
	...DefaultTheme,
	setup() {
		const route = useRoute();
		const initZoom = () => {
			mediumZoom(".main img", { background: "var(--vp-c-bg)" });
		};
		onMounted(() => {
			initZoom();
		});
		watch(
			() => route.path,
			() => nextTick(() => initZoom()),
		);
	},
};
