export default () => (
	<div
		style={{
			position: 'fixed',
			top: '53%',
			left: '50%',
			transform: 'translate(-50%, -53%)',
			pointerEvents: 'none'
		}}>
		<div className="h-1 w-96 bg-zinc-800 overflow-hidden rounded-full">
			<div className="animate-progress w-full h-full bg-zinc-50 origin-left-right"></div>
		</div>
	</div>
);
