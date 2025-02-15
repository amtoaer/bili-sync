<script lang="ts">
	import { onMount } from 'svelte';
	import Sidebar, { type VideoListModel } from '../components/Sidebar.svelte';
	import VideoList, { type VideoInfo } from '../components/VideoList.svelte';

	let videos: VideoInfo[] = [];
	let listModel: VideoListModel | null = null;
	let currentFilter: { key: keyof VideoListModel; id: number } | null = null;
	let loadingVideos = false;
	let sidebarOpen = true;
	let gridView = true;

	async function fetchListModel() {
		const res = await fetch('/api/video-list-models');
		if (res.ok) {
			listModel = await res.json();
		} else {
			console.error('加载列表结构失败');
		}
	}

	async function fetchVideos() {
		loadingVideos = true;
		let url = '/api/videos';
		if (currentFilter) {
			url += `?${currentFilter.key}=${currentFilter.id}`;
		}
		const res = await fetch(url);
		if (res.ok) {
			videos = await res.json();
		} else {
			console.error('加载视频列表失败');
		}
		loadingVideos = false;
	}

	onMount(async () => {
		await fetchListModel();
		await fetchVideos();
	});

	function selectFilter(key: keyof VideoListModel, id: number) {
		currentFilter = { key, id };
		fetchVideos();
	}

	function clearFilter() {
		currentFilter = null;
		fetchVideos();
	}

	function toggleSidebar() {
		sidebarOpen = !sidebarOpen;
	}

	function setGridView(value: boolean) {
		gridView = value;
	}
</script>

<!-- 整体背景和 container 样式 -->
<div class="min-h-screen bg-base-200">
	<main class="container mx-auto p-6">
		<div class="flex flex-col lg:flex-row gap-6">
			<aside class="w-full lg:w-64">
				<div class="card bg-base-100 shadow-lg">
					<Sidebar {listModel} {sidebarOpen} {toggleSidebar} {selectFilter} {clearFilter} />
				</div>
			</aside>
			<section class="flex-1">
				<div class="card bg-base-100 shadow-lg h-full">
					<VideoList {videos} {loadingVideos} {gridView} {setGridView} />
				</div>
			</section>
		</div>
	</main>
</div>
