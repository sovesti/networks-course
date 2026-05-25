const canvas = document.getElementById("{}");
const width = canvas.clientWidth;
const height = canvas.clientHeight;
if (canvas.width != width || canvas.height != height) {{
    canvas.width = width;
    canvas.height = height;
}}
const ctx = canvas.getContext("2d");
ctx.beginPath();
ctx.strokeStyle = "red";
ctx.lineWidth = {};
ctx.moveTo({}, {});
ctx.lineTo({}, {});
ctx.stroke();
ctx.closePath();
