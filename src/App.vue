<script setup lang="ts">
import { open, save } from "@tauri-apps/api/dialog";
import { ref, TextareaHTMLAttributes } from "vue";
import { MessagePlugin } from "tdesign-vue-next";
import { invoke } from "@tauri-apps/api/tauri";
import { emit, listen } from "@tauri-apps/api/event";

const apkgPath = ref("");
const voiceDir = ref("");

const chooseApkgFile = async () => {
  const selected = await open({
    filters: [
      {
        name: "牌组文件",
        extensions: ["apkg"],
      },
    ],
  });
  if (selected) {
    apkgPath.value = selected as string;
  }
};

const chooseVoiceDir = async () => {
  const selected = await open({
    directory: true,
  });
  if (selected) {
    voiceDir.value = selected as string;
  }
};

const start = async () => {
  if (apkgPath.value === "" || voiceDir.value === "") {
    MessagePlugin.error({
      content: "请先选择牌组文件和音频目录",
    });
    return;
  }
  try {
    const filePath = await save({
      filters: [
        {
          name: "牌组文件",
          extensions: ["apkg"],
        },
      ],
    });
    invoke("add_sound", {
      apkgPath: apkgPath.value,
      voiceDir: voiceDir.value,
      outApkgPath: filePath,
    });
  } catch (error) {
    MessagePlugin.error({
      content: "请选择目标文件",
    });
  }
};

let progressOutput = ref<HTMLTextAreaElement | null>(null);
const progressData = ref('');
listen('onUpdateProgress', (event) => {
  if (progressData.value !== '') {
    progressData.value= progressData.value+'\n';
  }
  progressData.value = progressData.value + event.payload;
  if(progressOutput.value){
    progressOutput.value.scrollTop = progressOutput.value?.scrollHeight;
  }
})

</script>

<template>
  <t-form id="form">
    <t-form-item label="牌组文件" name="apkgPath">
      <t-input placeholder="请选择牌组文件" v-model="apkgPath">
        <template #suffix>
          <t-button class="file-choose-btn" @click="chooseApkgFile">选择文件</t-button>
        </template>
      </t-input>
    </t-form-item>
    <t-form-item label="音频目录" name="voiceDir" initialData="123456">
      <t-input placeholder="请选择音频目录" v-model="voiceDir">
        <template #suffix>
          <t-button class="file-choose-btn" @click="chooseVoiceDir">选择目录</t-button>
        </template>
      </t-input>
    </t-form-item>
  </t-form>
  <textarea ref="progressOutput" readonly id="progress-output" v-model="progressData" />
  <t-button id="start" @click="start">开始处理</t-button>
</template>

<style lang="scss">
html,
body {
  height: 100%;
  width: 100%;
  margin: 0px;
}

#app {
  width: 100%;
  height: 100%;
  display: flex;
  flex-direction: column;
  align-items: center;
}

#form {
  margin-top: 20px;
  width: 80%;
}

#progress-output {
  margin: 20px 20px 20px 20px;
  height: 80%;
  width: 80%;
}

#start {
  position: relative;
  bottom: 10px;
}
</style>
