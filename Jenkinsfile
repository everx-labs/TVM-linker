@Library('infrastructure-jenkins-shared-library@master')_

G_gitcred = "LaninSSHgit"
G_docker_creds = 'dockerhubLanin'
G_promoted_branch = 'origin/master'
G_docker_src_image = null
G_docker_pub_image = null
G_dockerimage = null
G_buildstatus = "NotSet"
G_teststatus = "NotSet"
G_binversion = "NotSet"
C_PROJECT = "NotSet"
C_COMMITER = "NotSet"
C_HASH = "NotSet"
C_TEXT = "NotSet"
G_images = [:]
G_branches = [:]
G_params = null
G_docker_image = null
G_build = "none"
G_test = "none"
G_commit = ""

def isUpstream() {
    return currentBuild.getBuildCauses()[0]._class.toString() == 'hudson.model.Cause$UpstreamCause'
}

def buildImagesMap() {
    if (params.image_ton_types == '') {
        G_images.put('ton-types', "tonlabs/ton-types:tvm-linker-${GIT_COMMIT}")
    } else {
        G_images.put('ton-types', params.image_ton_types)
    }

    if (params.image_ton_labs_types == '') {
        G_images.put('ton-labs-types', "tonlabs/ton-labs-types:tvm-linker-${GIT_COMMIT}")
    } else {
        G_images.put('ton-labs-types', params.image_ton_labs_types)
    }

    if (params.image_ton_block == '') {
        G_images.put('ton-block', "tonlabs/ton-block:tvm-linker-${GIT_COMMIT}")
    } else {
        G_images.put('ton-block', params.image_ton_block)
    }

    if (params.image_ton_labs_block == '') {
        G_images.put('ton-labs-block', "tonlabs/ton-labs-block:tvm-linker-${GIT_COMMIT}")
    } else {
        G_images.put('ton-labs-block', params.image_ton_labs_block)
    }

    if (params.image_ton_vm == '') {
        G_images.put('ton-vm', "tonlabs/ton-vm:tvm-linker-${GIT_COMMIT}")
    } else {
        G_images.put('ton-vm', params.image_ton_vm)
    }

    if (params.image_ton_labs_vm == '') {
        G_images.put('ton-labs-vm', "tonlabs/ton-labs-vm:tvm-linker-${GIT_COMMIT}")
    } else {
        G_images.put('ton-labs-vm', params.image_ton_labs_vm)
    }

    if (params.image_ton_labs_abi == '') {
        G_images.put('ton-labs-abi', "tonlabs/ton-labs-abi:tvm-linker-${GIT_COMMIT}")
    } else {
        G_images.put('ton-labs-abi', params.image_ton_labs_abi)
    }

    if (params.image_ton_executor == '') {
        G_images.put('ton-executor', "tonlabs/ton-executor:tvm-linker-${GIT_COMMIT}")
    } else {
        G_images.put('ton-executor', params.image_ton_executor)
    }

    if (params.image_ton_sdk == '') {
        G_images.put('ton-sdk', "tonlabs/ton-sdk:tvm-linker-${GIT_COMMIT}")
    } else {
        G_images.put('ton-sdk', params.image_ton_sdk)
    }

    if (params.image_tvm_linker == '') {
        G_images.put('tvm-linker', "tonlabs/tvm_linker:${GIT_COMMIT}")
    } else {
        G_images.put('tvm-linker', params.image_tvm_linker)
    }
}

def buildBranchesMap() {
    if (params.branch_ton_types == '') {
        G_branches.put('ton-types', "master")
    } else {
        G_branches.put('ton-types', params.branch_ton_types)
    }
    
    if (params.branch_ton_labs_types == '') {
        G_branches.put('ton-labs-types', "release-candidate")
    } else {
        G_branches.put('ton-labs-types', params.branch_ton_labs_types)
    }

    if (params.branch_ton_block == '') {
        G_branches.put('ton-block', "master")
    } else {
        G_branches.put('ton-block', params.branch_ton_block)
    }

    if (params.branch_ton_labs_block == '') {
        G_branches.put('ton-labs-block', "release-candidate")
    } else {
        G_branches.put('ton-labs-block', params.branch_ton_labs_block)
    }

    if (params.branch_ton_vm == '') {
        G_branches.put('ton-vm', "master")
    } else {
        G_branches.put('ton-vm', params.branch_ton_vm)
    }

    if (params.branch_ton_labs_vm == '') {
        G_branches.put('ton-labs-vm', "release-candidate")
    } else {
        G_branches.put('ton-labs-vm', params.branch_ton_labs_vm)
    }

    if (params.branch_ton_labs_abi == '') {
        G_branches.put('ton-labs-abi', "master")
    } else {
        G_branches.put('ton-labs-abi', params.branch_ton_labs_abi)
    }

    if (params.branch_ton_executor == '') {
        G_branches.put('ton-executor', "master")
    } else {
        G_branches.put('ton-executor', params.branch_ton_executor)
    }

    if (params.branch_ton_sdk == '') {
        G_branches.put('ton-sdk', "master")
    } else {
        G_branches.put('ton-sdk', params.branch_ton_sdk)
    }

    if (params.branch_tvm_linker == '') {
        G_branches.put('tvm-linker', "${env.BRANCH_NAME}")
    } else {
        G_branches.put('tvm-linker', params.branch_tvm_linker)
    }

    if (params.branch_sol2tvm == '') {
        G_branches.put('sol2tvm', "master")
    } else {
        G_branches.put('sol2tvm', params.branch_sol2tvm)
    }
}

def buildParams() {
    buildImagesMap()
    buildBranchesMap()
    G_params = []
    params.each { key, value ->
        def item = null
        def nKey = key.toLowerCase().replaceAll('branch_', '').replaceAll('image_', '').replaceAll('_','-')
        if(key ==~ '^branch_(.)+') {
            item = string("name": key, "value": G_branches["${nKey}"])
        } else {
            if(key ==~ '^image_(.)+') {
                item = string("name": key, "value": G_images["${nKey}"])
            } else {
                if(key == 'common_version') {
                    item = string("name": 'key', "value": G_binversion)
                } else {
                    item = string("name": key, "value": value)
                }
            }
        }
        G_params.push(item)
    }
}

pipeline {
    parameters {
        string(
            name:'common_version',
            defaultValue: '',
            description: 'Common version'
        )
        string(
            name:'branch_ton_types',
            defaultValue: 'master',
            description: 'ton-types branch for dependency test'
        )
        string(
            name:'image_ton_types',
            defaultValue: '',
            description: 'ton-types image name'
        )
        string(
            name:'branch_ton_labs_types',
            defaultValue: '',
            description: 'ton-labs-types branch for dependency test'
        )
        string(
            name:'image_ton_labs_types',
            defaultValue: '',
            description: 'ton-labs-types image name'
        )
        string(
            name:'branch_ton_block',
            defaultValue: 'master',
            description: 'ton-block branch'
        )
        string(
            name:'image_ton_block',
            defaultValue: '',
            description: 'ton-block image name'
        )
        string(
            name:'branch_ton_labs_block',
            defaultValue: '',
            description: 'ton-labs-block branch'
        )
        string(
            name:'image_ton_labs_block',
            defaultValue: '',
            description: 'ton-labs-block image name'
        )
        string(
            name:'branch_ton_vm',
            defaultValue: 'master',
            description: 'ton-vm branch'
        )
        string(
            name:'image_ton_vm',
            defaultValue: '',
            description: 'ton-vm image name'
        )
        string(
            name:'branch_ton_labs_vm',
            defaultValue: '',
            description: 'ton-labs-vm branch'
        )
        string(
            name:'image_ton_labs_vm',
            defaultValue: '',
            description: 'ton-labs-vm image name'
        )
        string(
            name:'branch_ton_labs_abi',
            defaultValue: 'master',
            description: 'ton-labs-abi branch'
        )
        string(
            name:'image_ton_labs_abi',
            defaultValue: '',
            description: 'ton-labs-abi image name'
        )
        string(
            name:'branch_ton_executor',
            defaultValue: 'master',
            description: 'ton-executor branch'
        )
        string(
            name:'image_ton_executor',
            defaultValue: '',
            description: 'ton-executor image name'
        )
        string(
            name:'branch_tvm_linker',
            defaultValue: 'master',
            description: 'tvm-linker branch'
        )
        string(
            name:'image_tvm_linker',
            defaultValue: '',
            description: 'tvm-linker image name'
        )
        string(
            name:'branch_ton_sdk',
            defaultValue: 'master',
            description: 'ton-sdk branch'
        )
        string(
            name:'image_ton_sdk',
            defaultValue: '',
            description: 'ton-sdk image name'
        )
        string(
            name:'branch_sol2tvm',
            defaultValue: 'master',
            description: 'sol2tvm branch'
        )
    }
    agent {
        node {label 'master'}
    }
    tools {nodejs "Node12.8.0"}
    options {
        buildDiscarder logRotator(artifactDaysToKeepStr: '', artifactNumToKeepStr: '', daysToKeepStr: '', numToKeepStr: '10')
        disableConcurrentBuilds()
        parallelsAlwaysFailFast()
    }
    stages {
        stage('Versioning') {
            steps {
                script {
                    withAWS(credentials: 'CI_bucket_writer', region: 'eu-central-1') {
                        identity = awsIdentity()
                        s3Download bucket: 'sdkbinaries.tonlabs.io', file: 'version.json', force: true, path: 'version.json'
                    }
                    def folders = "./tvm_linker"
                    if(params.common_version) {
                        G_binversion = sh (script: "node tonVersion.js --set ${params.common_version} ${folders}", returnStdout: true).trim()
                    } else {
                        G_binversion = sh (script: "node tonVersion.js ${folders}", returnStdout: true).trim()
                    }


                    withAWS(credentials: 'CI_bucket_writer', region: 'eu-central-1') {
                        identity = awsIdentity()
                        s3Upload \
                            bucket: 'sdkbinaries.tonlabs.io', \
                            includePathPattern:'version.json', path: '', \
                            workingDir:'.'
                    }
                }
            }
        }
        stage('Prepare') {
            steps {
                script {
                    buildParams()
                    echo "${G_params}"
                }
            }
        }
        stage('Before stages') {
            when {
                expression {
                    return !isUpstream()
                }
            }
            steps {
                script {
                    def beforeParams = G_params
                    beforeParams.push(string("name": "project_name", "value": "tvm-linker"))
                    beforeParams.push(string("name": "stage", "value": "before"))
                    build job: 'Builder/build-flow', parameters: beforeParams
                }
            }
        }
        stage('Switch to file source') {
            steps {
                script {
                    sh """
(cat tvm_linker/Cargo.toml | \
sed 's/ton_types = .*/ton_types = { path = \"\\/tonlabs\\/ton-labs-types\" }/g' | \
sed 's/ton_block = .*/ton_block = { path = \"\\/tonlabs\\/ton-labs-block\" }/g' | \
sed 's/ton_abi = .*/ton_abi = { path = \"\\/tonlabs\\/ton-labs-abi\" }/g' | \
sed 's/ton_vm = .*/ton_vm = { path = \"\\/tonlabs\\/ton-labs-vm\", default-features = false }/g') > ./tvm_linker/tmp.toml
rm ./tvm_linker/Cargo.toml
mv ./tvm_linker/tmp.toml ./tvm_linker/Cargo.toml
                    """
                }
            }
        }
        stage('Build sources image') {
            steps {
                script {
                    G_docker_src_image = "tonlabs/tvm_linker:src-${GIT_COMMIT}"
                    docker.withRegistry('', G_docker_creds) {
                        sshagent (credentials: [G_gitcred]) {
                            withEnv(["DOCKER_BUILDKIT=1", "BUILD_INFO=src-${env.BUILD_TAG}:${GIT_COMMIT}"]) {
                                app_src_image = docker.build(
                                    "${G_docker_src_image}",
                                    "--pull --label \"git-commit=\${GIT_COMMIT}\" --target tvm-linker-src ."
                                )
                            }
                        }
                        app_src_image.push()
                    }
                }
            }
        }
        stage('Prepare sources for agents') {
            agent {
                dockerfile {
                    registryCredentialsId "${G_docker_creds}"
                    additionalBuildArgs "--pull --target linker-src " + 
                                        "--build-arg \"TON_LABS_TYPES_IMAGE=${G_images['ton-labs-types']}\" " +
                                        "--build-arg \"TON_LABS_BLOCK_IMAGE=${G_images['ton-labs-block']}\" " + 
                                        "--build-arg \"TON_LABS_VM_IMAGE=${G_images['ton-labs-vm']}\" " + 
                                        "--build-arg \"TON_LABS_ABI_IMAGE=${G_images['ton-labs-abi']}\" " + 
                                        "--build-arg \"TVM_LINKER_SRC_IMAGE=${G_docker_src_image}\""
                }
            }
            steps {
                script {
                    sh """
                        zip -9 -r linker-src.zip /tonlabs/*
                        chown jenkins:jenkins linker-src.zip
                    """
                    stash includes: '**/linker-src.zip', name: 'linker-src'
                }
            }
        }
        stage('Build') {
            failFast true
            parallel {
                stage('Parallel stages') {
                    when {
                        expression {
                            return !isUpstream()
                        }
                    }
                    steps {
                        script {
                            def intimeParams = G_params
                            intimeParams.push(string("name": "project_name", "value": "tvm-linker"))
                            intimeParams.push(string("name": "stage", "value": "in_time"))
                            build job: 'Builder/build-flow', parameters: intimeParams
                        }
                    }
                }
                stage ('Build docker image') {
                    steps {
                        script {
                            G_docker_pub_image = "tonlabs/tvm_linker:${GIT_COMMIT}"
                            docker.withRegistry('', G_docker_creds) {
                                sshagent (credentials: [G_gitcred]) {
                                    withEnv(["DOCKER_BUILDKIT=1", "BUILD_INFO=${env.BUILD_TAG}:${GIT_COMMIT}"]) {
                                        app_image = docker.build(
                                            "${G_docker_pub_image}",
                                            "--pull --label \"git-commit=\${GIT_COMMIT}\" --ssh default " + 
                                            "--build-arg \"RUST_IMAGE=${"rust:latest"}\" " + 
                                            "--build-arg \"TON_LABS_TYPES_IMAGE=${G_images['ton-labs-types']}\" " +
                                            "--build-arg \"TON_LABS_BLOCK_IMAGE=${G_images['ton-labs-block']}\" " + 
                                            "--build-arg \"TON_LABS_VM_IMAGE=${G_images['ton-labs-vm']}\" " + 
                                            "--build-arg \"TON_LABS_ABI_IMAGE=${G_images['ton-labs-abi']}\" " + 
                                            "--build-arg \"TVM_LINKER_SRC_IMAGE=${G_docker_src_image}\" " +
                                            "."
                                        )
                                    }
                                }
                                app_image.push()
                                def secTag = G_images['tvm-linker'].replaceAll('tonlabs/tvm_linker:','')
                                app_image.push(secTag)
                            }
                        }
                    }
                }
                stage('Build linux') {
                    /*when { 
                        branch 'master'
                    }*/
                    agent {
                        dockerfile {
                            registryCredentialsId "${G_docker_creds}"
                            additionalBuildArgs "--pull --target build-ton-compiler " + 
                                        "--build-arg \"TON_LABS_TYPES_IMAGE=${G_images['ton-labs-types']}\" " +
                                        "--build-arg \"TON_LABS_BLOCK_IMAGE=${G_images['ton-labs-block']}\" " + 
                                        "--build-arg \"TON_LABS_VM_IMAGE=${G_images['ton-labs-vm']}\" " + 
                                        "--build-arg \"TON_LABS_ABI_IMAGE=${G_images['ton-labs-abi']}\" " + 
                                        "--build-arg \"TVM_LINKER_SRC_IMAGE=${G_docker_src_image}\""
                        }
                    }
                    steps {
                        script {
                            withAWS(credentials: 'CI_bucket_writer', region: 'eu-central-1') {
                                identity = awsIdentity()
                                s3Download bucket: 'sdkbinaries.tonlabs.io', file: 'tvm_linker.json', force: true, path: 'tvm_linker.json'
                            }
                            sh 'node gzip.js ../../../../app/tvm_linker'
                            withAWS(credentials: 'CI_bucket_writer', region: 'eu-central-1') {
                                identity = awsIdentity()
                                s3Upload \
                                    bucket: 'sdkbinaries.tonlabs.io', \
                                    includePathPattern:'*.gz', path: 'tmp_linker', \
                                    workingDir:'.'
                                s3Upload \
                                    bucket: 'sdkbinaries.tonlabs.io', \
                                    includePathPattern:'tvm_linker.json', path: 'tmp_linker', \
                                    workingDir:'.'
                            }
                        }
                    }
                    post {
						cleanup {script{cleanWs notFailBuild: true}}
					}
                }
                stage('Build darwin') {
                    /*when { 
                        branch 'master'
                    }*/
                    agent {
                        label 'ios'
                    }
                    steps {
                        script {
                            withAWS(credentials: 'CI_bucket_writer', region: 'eu-central-1') {
                                identity = awsIdentity()
                                s3Download bucket: 'sdkbinaries.tonlabs.io', file: 'tvm_linker.json', force: true, path: 'tvm_linker.json'
                            }
                            def C_PATH = sh (script: 'pwd', returnStdout: true).trim()
                            
                            unstash 'linker-src'
                            sh """
                                unzip linker-src.zip
                                node pathFix.js tonlabs/ton-labs-block/Cargo.toml \"{ path = \\\"/tonlabs/\" \"{ path = \\\"${C_PATH}/tonlabs/\"
                                node pathFix.js tonlabs/ton-labs-vm/Cargo.toml \"{ path = \\\"/tonlabs/\" \"{ path = \\\"${C_PATH}/tonlabs/\"
                                node pathFix.js tonlabs/ton-labs-abi/Cargo.toml \"{ path = \\\"/tonlabs/\" \"{ path = \\\"${C_PATH}/tonlabs/\"
                                node pathFix.js tonlabs/tvm_linker/Cargo.toml \"{ path = \\\"/tonlabs/\" \"{ path = \\\"${C_PATH}/tonlabs/\"
                            """
                            dir('tonlabs') {
                                dir('tvm_linker') {
                                    sh """
                                        cargo update
                                        cargo build --release
                                        chmod a+x target/release/tvm_linker
                                    """
                                }
                            }
                            sh 'node gzip.js tonlabs/tvm_linker/target/release/tvm_linker'
                            withAWS(credentials: 'CI_bucket_writer', region: 'eu-central-1') {
                                identity = awsIdentity()
                                s3Upload \
                                    bucket: 'sdkbinaries.tonlabs.io', \
                                    includePathPattern:'*.gz', path: 'tmp_linker', \
                                    workingDir:'.'
                                s3Upload \
                                    bucket: 'sdkbinaries.tonlabs.io', \
                                    includePathPattern:'tvm_linker.json', path: 'tmp_linker', \
                                    workingDir:'.'
                            }
                        }
                    }
                    post {
						cleanup {script{cleanWs notFailBuild: true}}
					}
                }
                stage('Build windows') {
                    /*when { 
                        branch 'master'
                    }*/
                    agent {
                        label 'Win'
                    }
                    steps {
                        script {
                            def C_PATH = bat (script: '@echo off && echo %cd%', returnStdout: true).trim()
                            echo "${C_PATH}"
                            withAWS(credentials: 'CI_bucket_writer', region: 'eu-central-1') {
                                identity = awsIdentity()
                                s3Download bucket: 'sdkbinaries.tonlabs.io', file: 'tvm_linker.json', force: true, path: 'tvm_linker.json'
                            }
                            unstash 'linker-src'
                            bat """
                                unzip linker-src.zip
                                node pathFix.js tonlabs\\ton-labs-block\\Cargo.toml \"{ path = \\\"/tonlabs/\" \"{ path = \\\"${C_PATH}\\tonlabs\\\\\"
                                node pathFix.js tonlabs\\ton-labs-vm\\Cargo.toml \"{ path = \\\"/tonlabs/\" \"{ path = \\\"${C_PATH}\\tonlabs\\\\\"
                                node pathFix.js tonlabs\\ton-labs-abi\\Cargo.toml \"{ path = \\\"/tonlabs/\" \"{ path = \\\"${C_PATH}\\tonlabs\\\\\"
                                node pathFix.js tonlabs\\tvm_linker\\Cargo.toml \"{ path = \\\"/tonlabs/\" \"{ path = \\\"${C_PATH}\\tonlabs\\\\\"
                            """
                            dir('tonlabs') {
                                dir('tvm_linker') {
                                    bat """
                                        cargo update
                                        cargo build --release
                                        chmod a+x target/release/tvm_linker
                                    """
                                }
                            }

                            bat "node gzip.js tonlabs\\tvm_linker\\target\\release\\tvm_linker.exe"
                            withAWS(credentials: 'CI_bucket_writer', region: 'eu-central-1') {
                                identity = awsIdentity()
                                s3Upload \
                                    bucket: 'sdkbinaries.tonlabs.io', \
                                    includePathPattern:'*.gz', path: 'tmp_linker', \
                                    workingDir:'.'
                                s3Upload \
                                    bucket: 'sdkbinaries.tonlabs.io', \
                                    includePathPattern:'tvm_linker.json', path: 'tmp_linker', \
                                    workingDir:'.'
                            }
                        }
                    }
                    post {
						cleanup {script{cleanWs notFailBuild: true}}
					}
                }
            }
        }
        stage ('Test') {
            agent {
                docker {
                    image "${G_docker_pub_image}"
                    alwaysPull false
                    args '-u root'
                }
            }
            steps {
                script {
                    sh 'apk add python'
                    sh '/usr/bin/tvm_linker --version'
                    sh 'cd tvm_linker && python test_suite.py --linker-path=/usr/bin/tvm_linker'
                }
            }
        }
	    stage('Push docker-image') {
            steps {
                script {
                    docker.withRegistry('', G_docker_creds) {
                        app_image.push()
                    }
                }
            }
            post {
                failure {script{G_buildstatus = "failure"}}
            }
        }
        stage('Test in compiler-kit') {
            steps {
                script {
                    G_docker_pub_image = "tonlabs/tvm_linker:${GIT_COMMIT}"
                    def params = [
                      [$class: 'StringParameterValue', name: 'dockerimage_tvm_linker', value: "${G_docker_pub_image}"]
                    ]
                    build job : "Infrastructure/compilers/master", parameters : params
                }
            }
            post {
                success {
                    script{
                        G_buildstatus = "success"
                    }
                }
                failure {script{G_buildstatus = "failure"}}
            }
        }
        stage ('Tag as latest') {
            when {
                expression {
                    // GIT_BRANCH = 'origin/' + sh(returnStdout: true, script: 'git rev-parse --abbrev-ref HEAD').trim()
                    GIT_BRANCH = "origin/${BRANCH_NAME}"
                    return GIT_BRANCH == G_promoted_branch || params.FORCE_PROMOTE_LATEST
                }
            }
            steps {
                script {
                    docker.withRegistry('', G_docker_creds) {
                        docker.image("${G_docker_pub_image}").push('latest')
                        docker.image("${G_docker_src_image}").push('src-latest')
                    }
                }
            }
        }
        stage('After stages') {
            when {
                expression {
                    return !isUpstream()
                }
            }
            steps {
                script {
                    def afterParams = G_params
                    afterParams.push(string("name": "project_name", "value": "tvm-linker"))
                    afterParams.push(string("name": "stage", "value": "after"))
                    build job: 'Builder/build-flow', parameters: afterParams
                }
            }
        }
    }
    post {
        success {
            node ('master') {
                script {
                    withAWS(credentials: 'CI_bucket_writer', region: 'eu-central-1') {
                        identity = awsIdentity()
                        list = s3FindFiles(bucket: 'sdkbinaries.tonlabs.io', path: 'tmp_linker/', glob: '*')
                        for (def file : list) {
                            s3Copy fromBucket: 'sdkbinaries.tonlabs.io', fromPath: "tmp_linker/${file.path}", toBucket: 'sdkbinaries.tonlabs.io', toPath: "${file.path}"
                        }
                        s3Delete bucket: 'sdkbinaries.tonlabs.io', path: 'tmp_linker/'
                    }
                    def cause = "${currentBuild.getBuildCauses()}"
                    echo "${cause}"
                    if(!cause.matches('upstream')) {
                        withAWS(credentials: 'CI_bucket_writer', region: 'eu-central-1') {
                            identity = awsIdentity()
                            s3Download bucket: 'sdkbinaries.tonlabs.io', file: 'version.json', force: true, path: 'version.json'
                        }
                        sh """
                            echo const fs = require\\(\\'fs\\'\\)\\; > release.js
                            echo const ver = JSON.parse\\(fs.readFileSync\\(\\'version.json\\'\\, \\'utf8\\'\\)\\)\\; >> release.js
                            echo if\\(!ver.release\\) { throw new Error\\(\\'Empty release field\\'\\); } >> release.js
                            echo if\\(ver.candidate\\) { ver.release = ver.candidate\\; ver.candidate = \\'\\'\\; } >> release.js
                            echo fs.writeFileSync\\(\\'version.json\\', JSON.stringify\\(ver\\)\\)\\; >> release.js
                            cat release.js
                            cat version.json
                            node release.js
                        """
                        withAWS(credentials: 'CI_bucket_writer', region: 'eu-central-1') {
                            identity = awsIdentity()
                            s3Upload \
                                bucket: 'sdkbinaries.tonlabs.io', \
                                includePathPattern:'version.json', workingDir:'.'
                        }
                    }
                }
            }
        }
        failure {
            node ('master') {
                script {
                    withAWS(credentials: 'CI_bucket_writer', region: 'eu-central-1') {
                        identity = awsIdentity()
                        s3Delete bucket: 'sdkbinaries.tonlabs.io', path: 'tmp_linker/'
                    }
                    def cause = "${currentBuild.getBuildCauses()}"
                    echo "${cause}"
                    if(!cause.matches('upstream')) {
                        withAWS(credentials: 'CI_bucket_writer', region: 'eu-central-1') {
                            identity = awsIdentity()
                            s3Download bucket: 'sdkbinaries.tonlabs.io', file: 'version.json', force: true, path: 'version.json'
                        }
                        sh """
                            echo const fs = require\\(\\'fs\\'\\)\\; > decline.js
                            echo const ver = JSON.parse\\(fs.readFileSync\\(\\'version.json\\'\\, \\'utf8\\'\\)\\)\\; >> decline.js
                            echo if\\(!ver.release\\) { throw new Error\\(\\'Unable to set decline version\\'\\)\\; } >> decline.js
                            echo ver.candidate = \\'\\'\\; >> decline.js
                            echo fs.writeFileSync\\(\\'version.json\\', JSON.stringify\\(ver\\)\\)\\; >> decline.js
                            cat decline.js
                            cat version.json
                            node decline.js
                        """
                        withAWS(credentials: 'CI_bucket_writer', region: 'eu-central-1') {
                            identity = awsIdentity()
                            s3Upload \
                                bucket: 'sdkbinaries.tonlabs.io', \
                                includePathPattern:'version.json', workingDir:'.'
                        }
                    }
                }
            }
        }
        always {
            notifyTeam(
                buildstatus: G_buildstatus
            )
        }
        cleanup {
            script {
                cleanWs notFailBuild: true
            }
        }
    }
}
