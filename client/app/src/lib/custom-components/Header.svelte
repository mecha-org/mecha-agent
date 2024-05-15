<script lang="ts">
	export let title : String = "Connect to Mecha";
	import MeachIcon from '$lib/images/svgs/MecahIcon.svg';
</script>

<div class="flex flex-row items-center gap-2 py-3">
	<img src={MeachIcon} alt="mecha" /> 
	<h1 class="text-[26px]"> {title} </h1>
</div>