<script lang="ts">
	import { Badge } from '$lib/components/ui/badge/index.js';
	import { Button } from '$lib/components/ui/button/index.js';
	import { Label } from '$lib/components/ui/label/index.js';
	import { Separator } from '$lib/components/ui/separator/index.js';
	import { Switch } from '$lib/components/ui/switch/index.js';
	import type { FilterOption } from '$lib/types';

	let { value = $bindable(), disabled = false }: { value: FilterOption; disabled?: boolean } =
		$props();
</script>

<div class="space-y-6">
	<div class="space-y-4">
		<Label>流质量过滤</Label>
		<p class="text-muted-foreground text-sm">
			根据质量过滤视频音频流，条件过于严苛可能导致视频无可用流
		</p>
		<div class="grid grid-cols-1 gap-6 lg:grid-cols-2">
			<div class="space-y-2">
				<Label for="video-max-quality">最高视频质量</Label>
				<select
					id="video-max-quality"
					class="border-input bg-background ring-offset-background placeholder:text-muted-foreground focus-visible:ring-ring flex h-10 w-full rounded-md border px-3 py-2 text-sm file:border-0 file:bg-transparent file:text-sm file:font-medium focus-visible:ring-2 focus-visible:ring-offset-2 focus-visible:outline-none disabled:cursor-not-allowed disabled:opacity-50"
					bind:value={value.video_max_quality}
					{disabled}
				>
					<option value="Quality360p">360p</option>
					<option value="Quality480p">480p</option>
					<option value="Quality720p">720p</option>
					<option value="Quality1080p">1080p</option>
					<option value="Quality1080pPLUS">1080p+</option>
					<option value="Quality1080p60">1080p60</option>
					<option value="Quality4k">4K</option>
					<option value="QualityHdr">HDR</option>
					<option value="QualityDolby">杜比视界</option>
					<option value="Quality8k">8K</option>
				</select>
			</div>
			<div class="space-y-2">
				<Label for="video-min-quality">最低视频质量</Label>
				<select
					id="video-min-quality"
					class="border-input bg-background ring-offset-background placeholder:text-muted-foreground focus-visible:ring-ring flex h-10 w-full rounded-md border px-3 py-2 text-sm file:border-0 file:bg-transparent file:text-sm file:font-medium focus-visible:ring-2 focus-visible:ring-offset-2 focus-visible:outline-none disabled:cursor-not-allowed disabled:opacity-50"
					bind:value={value.video_min_quality}
					{disabled}
				>
					<option value="Quality360p">360p</option>
					<option value="Quality480p">480p</option>
					<option value="Quality720p">720p</option>
					<option value="Quality1080p">1080p</option>
					<option value="Quality1080pPLUS">1080p+</option>
					<option value="Quality1080p60">1080p60</option>
					<option value="Quality4k">4K</option>
					<option value="QualityHdr">HDR</option>
					<option value="QualityDolby">杜比视界</option>
					<option value="Quality8k">8K</option>
				</select>
			</div>
			<div class="space-y-2">
				<Label for="audio-max-quality">最高音频质量</Label>
				<select
					id="audio-max-quality"
					class="border-input bg-background ring-offset-background placeholder:text-muted-foreground focus-visible:ring-ring flex h-10 w-full rounded-md border px-3 py-2 text-sm file:border-0 file:bg-transparent file:text-sm file:font-medium focus-visible:ring-2 focus-visible:ring-offset-2 focus-visible:outline-none disabled:cursor-not-allowed disabled:opacity-50"
					bind:value={value.audio_max_quality}
					{disabled}
				>
					<option value="Quality64k">64k</option>
					<option value="Quality132k">132k</option>
					<option value="Quality192k">192k</option>
					<option value="QualityDolby">杜比全景声</option>
					<option value="QualityHiRES">Hi-RES</option>
				</select>
			</div>
			<div class="space-y-2">
				<Label for="audio-min-quality">最低音频质量</Label>
				<select
					id="audio-min-quality"
					class="border-input bg-background ring-offset-background placeholder:text-muted-foreground focus-visible:ring-ring flex h-10 w-full rounded-md border px-3 py-2 text-sm file:border-0 file:bg-transparent file:text-sm file:font-medium focus-visible:ring-2 focus-visible:ring-offset-2 focus-visible:outline-none disabled:cursor-not-allowed disabled:opacity-50"
					bind:value={value.audio_min_quality}
					{disabled}
				>
					<option value="Quality64k">64k</option>
					<option value="Quality132k">132k</option>
					<option value="Quality192k">192k</option>
					<option value="QualityDolby">杜比全景声</option>
					<option value="QualityHiRES">Hi-RES</option>
				</select>
			</div>
		</div>
	</div>

	<Separator />

	<div class="space-y-4">
		<Label>视频编码格式偏好（按优先级排序）</Label>
		<p class="text-muted-foreground text-sm">排在前面的编码格式优先级更高</p>
		<div class="space-y-2">
			{#each value.codecs as codec, index (index)}
				<div class="flex items-center space-x-2 rounded-lg border p-3">
					<Badge variant="secondary">{index + 1}</Badge>
					<span class="flex-1 font-medium">{codec}</span>
					<div class="flex space-x-1">
						<Button
							type="button"
							size="sm"
							variant="outline"
							disabled={disabled || index === 0}
							onclick={() => {
								const codecs = [...value.codecs];
								[codecs[index - 1], codecs[index]] = [codecs[index], codecs[index - 1]];
								value.codecs = codecs;
							}}
						>
							↑
						</Button>
						<Button
							type="button"
							size="sm"
							variant="outline"
							disabled={disabled || index === value.codecs.length - 1}
							onclick={() => {
								const codecs = [...value.codecs];
								[codecs[index], codecs[index + 1]] = [codecs[index + 1], codecs[index]];
								value.codecs = codecs;
							}}
						>
							↓
						</Button>
						<Button
							type="button"
							size="sm"
							variant="destructive"
							{disabled}
							onclick={() => (value.codecs = value.codecs.filter((_, i) => i !== index))}
						>
							×
						</Button>
					</div>
				</div>
			{/each}

			{#if value.codecs.length < 3}
				<div class="space-y-2">
					<Label>添加编码格式</Label>
					<div class="flex gap-2">
						{#each ['AV1', 'HEV', 'AVC'] as codec (codec)}
							{#if !value.codecs.includes(codec)}
								<Button
									type="button"
									size="sm"
									variant="outline"
									{disabled}
									onclick={() => (value.codecs = [...value.codecs, codec])}
								>
									+ {codec}
								</Button>
							{/if}
						{/each}
					</div>
				</div>
			{/if}
		</div>
	</div>

	<Separator />

	<div class="space-y-4">
		<Label>特殊流排除选项</Label>
		<p class="text-muted-foreground text-sm">排除某些类型的特殊流</p>
		<div class="flex items-center space-x-2">
			<Switch id="no-dolby-video" bind:checked={value.no_dolby_video} {disabled} />
			<Label for="no-dolby-video">排除杜比视界视频</Label>
		</div>
		<div class="flex items-center space-x-2">
			<Switch id="no-dolby-audio" bind:checked={value.no_dolby_audio} {disabled} />
			<Label for="no-dolby-audio">排除杜比全景声音频</Label>
		</div>
		<div class="flex items-center space-x-2">
			<Switch id="no-hdr" bind:checked={value.no_hdr} {disabled} />
			<Label for="no-hdr">排除HDR视频</Label>
		</div>
		<div class="flex items-center space-x-2">
			<Switch id="no-hires" bind:checked={value.no_hires} {disabled} />
			<Label for="no-hires">排除Hi-RES音频</Label>
		</div>
	</div>
</div>
