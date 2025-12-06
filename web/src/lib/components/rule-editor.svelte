<script lang="ts">
	import { Button } from '$lib/components/ui/button/index.js';
	import { Input } from '$lib/components/ui/input/index.js';
	import { Label } from '$lib/components/ui/label/index.js';
	import { Checkbox } from '$lib/components/ui/checkbox/index.js';
	import * as Card from '$lib/components/ui/card/index.js';
	import { Badge } from '$lib/components/ui/badge/index.js';
	import PlusIcon from '@lucide/svelte/icons/plus';
	import MinusIcon from '@lucide/svelte/icons/minus';
	import XIcon from '@lucide/svelte/icons/x';
	import type { Rule, RuleTarget, Condition } from '$lib/types';
	import { onMount } from 'svelte';

	interface Props {
		rule: Rule | null;
		onRuleChange: (rule: Rule | null) => void;
	}

	let { rule, onRuleChange }: Props = $props();

	const FIELD_OPTIONS = [
		{ value: 'title', label: '标题' },
		{ value: 'tags', label: '标签' },
		{ value: 'favTime', label: '收藏时间' },
		{ value: 'pubTime', label: '发布时间' },
		{ value: 'pageCount', label: '视频分页数量' }
	];

	const getOperatorOptions = (field: string) => {
		switch (field) {
			case 'title':
			case 'tags':
				return [
					{ value: 'equals', label: '等于' },
					{ value: 'contains', label: '包含' },
					{ value: 'icontains', label: '包含（不区分大小写）' },
					{ value: 'prefix', label: '以...开头' },
					{ value: 'suffix', label: '以...结尾' },
					{ value: 'matchesRegex', label: '匹配正则' }
				];
			case 'pageCount':
				return [
					{ value: 'equals', label: '等于' },
					{ value: 'greaterThan', label: '大于' },
					{ value: 'lessThan', label: '小于' },
					{ value: 'between', label: '范围' }
				];
			case 'favTime':
			case 'pubTime':
				return [
					{ value: 'equals', label: '等于' },
					{ value: 'greaterThan', label: '晚于' },
					{ value: 'lessThan', label: '早于' },
					{ value: 'between', label: '时间范围' }
				];
			default:
				return [];
		}
	};

	interface LocalCondition {
		field: string;
		operator: string;
		value: string;
		value2?: string;
		isNot: boolean;
	}

	interface LocalAndGroup {
		conditions: LocalCondition[];
	}

	let localRule: LocalAndGroup[] = $state([]);

	onMount(() => {
		if (rule && rule.length > 0) {
			localRule = rule.map((andGroup) => ({
				conditions: andGroup.map((target) => convertRuleTargetToLocal(target))
			}));
		} else {
			localRule = [];
		}
	});

	function convertRuleTargetToLocal(target: RuleTarget<string | number | Date>): LocalCondition {
		if (typeof target.rule === 'object' && 'field' in target.rule) {
			// 嵌套的 not
			const innerCondition = convertRuleTargetToLocal(target.rule);
			return {
				...innerCondition,
				isNot: true
			};
		}
		const condition = target.rule as Condition<string | number | Date>;
		let value = '';
		let value2 = '';
		if (Array.isArray(condition.value)) {
			value = String(condition.value[0] || '');
			value2 = String(condition.value[1] || '');
		} else {
			value = String(condition.value || '');
		}
		return {
			field: target.field,
			operator: condition.operator,
			value,
			value2,
			isNot: false
		};
	}

	function convertLocalToRule(): Rule | null {
		if (localRule.length === 0) return null;
		return localRule.map((andGroup) =>
			andGroup.conditions.map((condition) => {
				let value: string | number | Date | (string | number | Date)[];
				if (condition.field === 'pageCount') {
					if (condition.operator === 'between') {
						value = [parseInt(condition.value) || 0, parseInt(condition.value2 || '0') || 0];
					} else {
						value = parseInt(condition.value) || 0;
					}
				} else if (condition.field === 'favTime' || condition.field === 'pubTime') {
					if (condition.operator === 'between') {
						value = [condition.value, condition.value2 || ''];
					} else {
						value = condition.value;
					}
				} else {
					if (condition.operator === 'between') {
						value = [condition.value, condition.value2 || ''];
					} else {
						value = condition.value;
					}
				}
				const conditionObj: Condition<string | number | Date> = {
					operator: condition.operator,
					value
				};

				let target: RuleTarget<string | number | Date> = {
					field: condition.field,
					rule: conditionObj
				};
				if (condition.isNot) {
					target = {
						field: 'not',
						rule: target
					};
				}
				return target;
			})
		);
	}

	function addAndGroup() {
		localRule.push({ conditions: [] });
		onRuleChange?.(convertLocalToRule());
	}

	function removeAndGroup(index: number) {
		localRule.splice(index, 1);
		onRuleChange?.(convertLocalToRule());
	}

	function addCondition(groupIndex: number) {
		localRule[groupIndex].conditions.push({
			field: 'title',
			operator: 'contains',
			value: '',
			isNot: false
		});
		onRuleChange?.(convertLocalToRule());
	}

	function removeCondition(groupIndex: number, conditionIndex: number) {
		localRule[groupIndex].conditions.splice(conditionIndex, 1);
		onRuleChange?.(convertLocalToRule());
	}

	function updateCondition(
		groupIndex: number,
		conditionIndex: number,
		field: string,
		value: string
	) {
		const condition = localRule[groupIndex].conditions[conditionIndex];
		if (field === 'field') {
			condition.field = value;
			const operators = getOperatorOptions(value);
			condition.operator = operators[0]?.value || 'equals';
			condition.value = '';
			condition.value2 = '';
		} else if (field === 'operator') {
			condition.operator = value;
			// 如果切换到/从 between 操作符，重置值
			if (value === 'between') {
				condition.value2 = condition.value2 || '';
			}
		} else if (field === 'value') {
			condition.value = value;
		} else if (field === 'value2') {
			condition.value2 = value;
		} else if (field === 'isNot') {
			condition.isNot = value === 'true';
		}
		onRuleChange?.(convertLocalToRule());
	}

	function clearRules() {
		localRule = [];
		onRuleChange?.(convertLocalToRule());
	}
</script>

<div class="space-y-4">
	<div class="flex items-center justify-between">
		<Label class="text-sm font-medium">过滤规则</Label>
		<div class="flex gap-2">
			{#if localRule.length > 0}
				<Button size="sm" variant="outline" onclick={clearRules}>清空规则</Button>
			{/if}
			<Button size="sm" onclick={addAndGroup}>
				<PlusIcon class="mr-1 h-3 w-3" />
				添加规则组
			</Button>
		</div>
	</div>

	{#if localRule.length === 0}
		<div class="border-muted-foreground/25 rounded-lg border-2 border-dashed p-8 text-center">
			<p class="text-muted-foreground mb-4 text-sm">暂无过滤规则，将下载所有视频</p>
			<Button size="sm" onclick={addAndGroup}>
				<PlusIcon class="mr-1 h-3 w-3" />
				添加第一个规则组
			</Button>
		</div>
	{:else}
		<div class="space-y-4">
			{#each localRule as andGroup, groupIndex (groupIndex)}
				<Card.Root>
					<Card.Header>
						<div class="flex items-center justify-between">
							<div class="flex items-center gap-2">
								<Badge variant="secondary">规则组 {groupIndex + 1}</Badge>
							</div>
							<Button
								size="sm"
								variant="ghost"
								onclick={() => removeAndGroup(groupIndex)}
								class="h-7 w-7 p-0"
							>
								<XIcon class="h-3 w-3" />
							</Button>
						</div>
					</Card.Header>
					<Card.Content class="space-y-3">
						{#each andGroup.conditions as condition, conditionIndex (conditionIndex)}
							<div class="space-y-3 rounded-lg border p-4">
								<div class="flex items-center justify-between">
									<Badge variant="secondary">条件 {conditionIndex + 1}</Badge>
									<Button
										size="sm"
										variant="ghost"
										onclick={() => removeCondition(groupIndex, conditionIndex)}
										class="h-7 w-7 p-0"
									>
										<MinusIcon class="h-3 w-3" />
									</Button>
								</div>

								<!-- 取反选项 -->
								<div class="flex items-center space-x-2">
									<Checkbox
										id={`not-${groupIndex}-${conditionIndex}`}
										checked={condition.isNot}
										onCheckedChange={(checked) =>
											updateCondition(
												groupIndex,
												conditionIndex,
												'isNot',
												checked ? 'true' : 'false'
											)}
									/>
									<Label for={`not-${groupIndex}-${conditionIndex}`} class="text-sm">
										取反（NOT）
									</Label>
								</div>

								<!-- 字段和操作符 -->
								<div class="grid grid-cols-2 gap-3">
									<!-- 字段选择 -->
									<div>
										<Label class="text-muted-foreground text-xs">字段</Label>
										<select
											class="border-input bg-background placeholder:text-muted-foreground focus-visible:ring-ring flex h-9 w-full rounded-md border px-3 py-1 text-sm shadow-sm transition-colors file:border-0 file:bg-transparent file:text-sm file:font-medium focus-visible:ring-1 focus-visible:outline-none disabled:cursor-not-allowed disabled:opacity-50"
											value={condition.field}
											onchange={(e) =>
												updateCondition(groupIndex, conditionIndex, 'field', e.currentTarget.value)}
										>
											{#each FIELD_OPTIONS as option (option.value)}
												<option value={option.value}>{option.label}</option>
											{/each}
										</select>
									</div>

									<!-- 操作符选择 -->
									<div>
										<Label class="text-muted-foreground text-xs">操作符</Label>
										<select
											class="border-input bg-background placeholder:text-muted-foreground focus-visible:ring-ring flex h-9 w-full rounded-md border px-3 py-1 text-sm shadow-sm transition-colors file:border-0 file:bg-transparent file:text-sm file:font-medium focus-visible:ring-1 focus-visible:outline-none disabled:cursor-not-allowed disabled:opacity-50"
											value={condition.operator}
											onchange={(e) =>
												updateCondition(
													groupIndex,
													conditionIndex,
													'operator',
													e.currentTarget.value
												)}
										>
											{#each getOperatorOptions(condition.field) as option (option.value)}
												<option value={option.value}>{option.label}</option>
											{/each}
										</select>
									</div>
								</div>

								<!-- 值输入 -->
								<div>
									<Label class="text-muted-foreground text-xs">值</Label>
									{#if condition.operator === 'between'}
										<div class="grid grid-cols-2 gap-2">
											{#if condition.field === 'pageCount'}
												<Input
													type="number"
													placeholder="最小值"
													class="h-9"
													value={condition.value}
													oninput={(e) =>
														updateCondition(
															groupIndex,
															conditionIndex,
															'value',
															e.currentTarget.value
														)}
												/>
												<Input
													type="number"
													placeholder="最大值"
													class="h-9"
													value={condition.value2 || ''}
													oninput={(e) =>
														updateCondition(
															groupIndex,
															conditionIndex,
															'value2',
															e.currentTarget.value
														)}
												/>
											{:else if condition.field === 'favTime' || condition.field === 'pubTime'}
												<Input
													type="datetime-local"
													placeholder="开始时间"
													class="h-9"
													value={condition.value}
													oninput={(e) =>
														updateCondition(
															groupIndex,
															conditionIndex,
															'value',
															e.currentTarget.value + ':00' // 前端选择器只能精确到分钟，此处附加额外的 :00 以满足后端传参条件
														)}
												/>
												<Input
													type="datetime-local"
													placeholder="结束时间"
													class="h-9"
													value={condition.value2 || ''}
													oninput={(e) =>
														updateCondition(
															groupIndex,
															conditionIndex,
															'value2',
															e.currentTarget.value + ':00'
														)}
												/>
											{:else}
												<Input
													type="text"
													placeholder="起始值"
													class="h-9"
													value={condition.value}
													oninput={(e) =>
														updateCondition(
															groupIndex,
															conditionIndex,
															'value',
															e.currentTarget.value
														)}
												/>
												<Input
													type="text"
													placeholder="结束值"
													class="h-9"
													value={condition.value2 || ''}
													oninput={(e) =>
														updateCondition(
															groupIndex,
															conditionIndex,
															'value2',
															e.currentTarget.value
														)}
												/>
											{/if}
										</div>
									{:else if condition.field === 'pageCount'}
										<Input
											type="number"
											placeholder="输入数值"
											class="h-9"
											value={condition.value}
											oninput={(e) =>
												updateCondition(groupIndex, conditionIndex, 'value', e.currentTarget.value)}
										/>
									{:else if condition.field === 'favTime' || condition.field === 'pubTime'}
										<Input
											type="datetime-local"
											placeholder="选择时间"
											class="h-9"
											value={condition.value}
											oninput={(e) =>
												updateCondition(
													groupIndex,
													conditionIndex,
													'value',
													e.currentTarget.value + ':00'
												)}
										/>
									{:else}
										<Input
											type="text"
											placeholder="输入文本"
											class="h-9"
											value={condition.value}
											oninput={(e) =>
												updateCondition(groupIndex, conditionIndex, 'value', e.currentTarget.value)}
										/>
									{/if}
								</div>
							</div>
						{/each}

						<Button
							size="sm"
							variant="outline"
							onclick={() => addCondition(groupIndex)}
							class="w-full"
						>
							<PlusIcon class="mr-1 h-3 w-3" />
							添加条件
						</Button>
					</Card.Content>
				</Card.Root>
			{/each}
		</div>
	{/if}

	{#if localRule.length > 0}
		<div class="text-muted-foreground bg-muted/50 rounded p-3 text-xs">
			<p class="mb-1 font-medium">规则说明：</p>
			<ul class="space-y-1">
				<li>• 多个规则组之间是"或"的关系，同一规则组内的条件是"且"的关系</li>
				<li>
					• 规则内配置的时间不包含时区，在处理时会默认应用<strong>服务器时区</strong
					>，不受浏览器影响
				</li>
			</ul>
		</div>
	{/if}
</div>
