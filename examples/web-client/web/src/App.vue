<script setup lang="ts">
import { ref, watch } from 'vue'
import { ws_client, receive_msg, send_msg, Stream } from 'stream-ws-web-client-example-wasm'

const text = ref("")
const result = ref("")
const connected = ref(false)
const receiving = ref(false)

let stream: Stream | null = null

function connect() {
  try {
    stream = ws_client()
  }
  catch (e) {
    console.error(e)
    return
  }
  connected.value = true
}

async function send() {
  if (stream == null) {
    reset()
    return
  }
  await send_msg(stream, text.value)
}

async function receive() {
  if (stream == null) {
    reset()
    return
  }
  while (receiving.value) {
    try {
      const msg = await receive_msg(stream, 64)
      result.value += msg
    }
    catch (e) {
      console.error(e)
      reset()
      return
    }
  }
}

function reset() {
  receiving.value = false
  connected.value = false
  stream = null
}

watch(receiving, function (value) {
  if (value) {
    receive()
  }
})

</script>

<template>
  <div>
    <button type="button" @click="connect">Connect</button>
    <button type="button" @click="send">Send</button>
    <button type="button" @click="receiving = !receiving">{{ receiving ? "Stop receiving" : "Receive" }}</button>
    <button type="button" @click="result = ''">Clear</button>
    <p>{{ connected ? "Connected" : "Disconnected" }}</p>
  </div>
  <div>
    <input v-model="text">
    <br />
    <span style="white-space: pre;">{{ result }}</span>
  </div>
</template>

<style scoped>
.read-the-docs {
  color: #888;
}
</style>
