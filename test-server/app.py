from flask import Flask, request, jsonify
import pika
import json
import os
import uuid
import base64
from datetime import datetime
import logging

# 配置日誌
logging.basicConfig(level=logging.INFO, format='%(asctime)s - %(name)s - %(levelname)s - %(message)s')
logger = logging.getLogger(__name__)

app = Flask(__name__)

# RabbitMQ 連接配置
RABBITMQ_HOST = os.environ.get('RABBITMQ_HOST', 'localhost')
RABBITMQ_PORT = int(os.environ.get('RABBITMQ_PORT', 5672))
RABBITMQ_USER = os.environ.get('RABBITMQ_USER', 'guest')
RABBITMQ_PASS = os.environ.get('RABBITMQ_PASS', 'guest')
RABBITMQ_VHOST = os.environ.get('RABBITMQ_VHOST', '/')

# 交換機和隊列配置
EXCHANGE_NAME = 'backtest_exchange'
BACKTEST_QUEUE = 'backtest_queue'
STRATEGY_QUEUE = 'strategy_queue'
DATA_QUEUE = 'data_queue'

# 建立 RabbitMQ 連接
def get_rabbitmq_connection():
    credentials = pika.PlainCredentials(RABBITMQ_USER, RABBITMQ_PASS)
    parameters = pika.ConnectionParameters(
        host=RABBITMQ_HOST,
        port=RABBITMQ_PORT,
        virtual_host=RABBITMQ_VHOST,
        credentials=credentials,
        heartbeat=600,
        blocked_connection_timeout=300
    )
    return pika.BlockingConnection(parameters)

# 初始化 RabbitMQ 連接和通道
def initialize_rabbitmq():
    try:
        connection = get_rabbitmq_connection()
        channel = connection.channel()
        
        # 宣告交換機
        channel.exchange_declare(
            exchange=EXCHANGE_NAME,
            exchange_type='topic',
            durable=True
        )
        
        # 宣告隊列
        channel.queue_declare(queue=BACKTEST_QUEUE, durable=True)
        channel.queue_declare(queue=STRATEGY_QUEUE, durable=True)
        channel.queue_declare(queue=DATA_QUEUE, durable=True)
        
        # 綁定隊列到交換機
        channel.queue_bind(
            exchange=EXCHANGE_NAME,
            queue=BACKTEST_QUEUE,
            routing_key='backtest.*'
        )
        channel.queue_bind(
            exchange=EXCHANGE_NAME,
            queue=STRATEGY_QUEUE,
            routing_key='strategy.*'
        )
        channel.queue_bind(
            exchange=EXCHANGE_NAME,
            queue=DATA_QUEUE,
            routing_key='data.*'
        )
        
        logger.info("RabbitMQ 初始化成功")
        return connection, channel
    except Exception as e:
        logger.error(f"RabbitMQ 初始化失敗: {str(e)}")
        raise

# 生成唯一消息 ID
def generate_message_id():
    return str(uuid.uuid4())

# 建立通用消息格式
def create_message(msg_type, payload, correlation_id=None):
    return {
        "message_id": generate_message_id(),
        "message_type": msg_type,
        "timestamp": datetime.now().isoformat(),
        "correlation_id": correlation_id or generate_message_id(),
        "payload": payload
    }

# 發送消息到 RabbitMQ
def send_message(channel, routing_key, message):
    try:
        channel.basic_publish(
            exchange=EXCHANGE_NAME,
            routing_key=routing_key,
            body=json.dumps(message),
            properties=pika.BasicProperties(
                delivery_mode=2,  # 持久化消息
                content_type='application/json',
                message_id=message['message_id'],
                correlation_id=message['correlation_id'],
                timestamp=int(datetime.now().timestamp())
            )
        )
        logger.info(f"消息已發送: {routing_key} -> {message['message_id']}")
        return True
    except Exception as e:
        logger.error(f"發送消息失敗: {str(e)}")
        return False

# ------ 消息模板 ------

# 回測相關消息模板
def create_backtest_request(strategy_id, start_date, end_date, initial_capital, instruments):
    payload = {
        "strategy_id": strategy_id,
        "config": {
            "start_date": start_date,
            "end_date": end_date,
            "initial_capital": initial_capital,
            "instruments": instruments,
            "execution_settings": {
                "slippage": 0.001,
                "commission": 0.0003
            },
            "risk_settings": {
                "max_position_size": 0.2,
                "max_drawdown": 0.1
            }
        }
    }
    return create_message("backtest.request", payload)

# 策略相關消息模板
def create_strategy_upload(strategy_id, strategy_name, strategy_code, version="1.0"):
    payload = {
        "strategy_id": strategy_id,
        "name": strategy_name,
        "version": version,
        "code": strategy_code,
        "parameters": {},
        "tags": ["example", "test"]
    }
    return create_message("strategy.upload", payload)

# 數據相關消息模板
def create_data_request(instrument_id, start_date, end_date, frequency="1d"):
    payload = {
        "instrument_id": instrument_id,
        "start_date": start_date,
        "end_date": end_date,
        "frequency": frequency
    }
    return create_message("data.request", payload)

# 檔案上傳消息模板
def create_file_upload_message(file_name, file_content_base64, file_type, related_id=None):
    payload = {
        "file_name": file_name,
        "file_content": file_content_base64,
        "file_type": file_type,
        "file_size": len(base64.b64decode(file_content_base64)),
        "related_id": related_id,
        "upload_time": datetime.now().isoformat()
    }
    return create_message("file.upload", payload)

# ------ HTTP API 端點 ------

@app.route('/health', methods=['GET'])
def health_check():
    return jsonify({"status": "ok", "timestamp": datetime.now().isoformat()})

@app.route('/api/backtest', methods=['POST'])
def backtest_request():
    try:
        data = request.json
        
        # 檢查必要參數
        required_fields = ['strategy_id', 'start_date', 'end_date', 'initial_capital', 'instruments']
        for field in required_fields:
            if field not in data:
                return jsonify({"error": f"缺少必要參數: {field}"}), 400
        
        # 創建消息
        message = create_backtest_request(
            data['strategy_id'],
            data['start_date'],
            data['end_date'],
            data['initial_capital'],
            data['instruments']
        )
        
        # 連接 RabbitMQ 並發送消息
        connection, channel = initialize_rabbitmq()
        result = send_message(channel, 'backtest.request', message)
        connection.close()
        
        if result:
            return jsonify({
                "message": "回測請求已發送",
                "message_id": message['message_id'],
                "correlation_id": message['correlation_id']
            })
        else:
            return jsonify({"error": "發送回測請求失敗"}), 500
    
    except Exception as e:
        return jsonify({"error": str(e)}), 500

@app.route('/api/strategy/upload', methods=['POST'])
def strategy_upload():
    try:
        # 檢查是否有檔案或JSON資料
        if 'file' in request.files:
            # 從文件處理
            file = request.files['file']
            strategy_id = request.form.get('strategy_id', generate_message_id())
            strategy_name = request.form.get('strategy_name', file.filename)
            version = request.form.get('version', '1.0')
            
            # 讀取檔案內容
            strategy_code = file.read().decode('utf-8')
            
        elif request.is_json:
            # 從JSON處理
            data = request.json
            strategy_id = data.get('strategy_id', generate_message_id())
            strategy_name = data.get('strategy_name', f"Strategy_{strategy_id}")
            strategy_code = data.get('code', '')
            version = data.get('version', '1.0')
        else:
            return jsonify({"error": "無效的請求格式，請提供JSON或檔案"}), 400
        
        # 創建消息
        message = create_strategy_upload(strategy_id, strategy_name, strategy_code, version)
        
        # 連接 RabbitMQ 並發送消息
        connection, channel = initialize_rabbitmq()
        result = send_message(channel, 'strategy.upload', message)
        connection.close()
        
        if result:
            return jsonify({
                "message": "策略已上傳",
                "strategy_id": strategy_id,
                "message_id": message['message_id']
            })
        else:
            return jsonify({"error": "發送策略請求失敗"}), 500
    
    except Exception as e:
        return jsonify({"error": str(e)}), 500

@app.route('/api/data/request', methods=['POST'])
def data_request():
    try:
        data = request.json
        
        # 檢查必要參數
        required_fields = ['instrument_id', 'start_date', 'end_date']
        for field in required_fields:
            if field not in data:
                return jsonify({"error": f"缺少必要參數: {field}"}), 400
        
        frequency = data.get('frequency', '1d')
        
        # 創建消息
        message = create_data_request(
            data['instrument_id'],
            data['start_date'],
            data['end_date'],
            frequency
        )
        
        # 連接 RabbitMQ 並發送消息
        connection, channel = initialize_rabbitmq()
        result = send_message(channel, 'data.request', message)
        connection.close()
        
        if result:
            return jsonify({
                "message": "數據請求已發送",
                "message_id": message['message_id']
            })
        else:
            return jsonify({"error": "發送數據請求失敗"}), 500
    
    except Exception as e:
        return jsonify({"error": str(e)}), 500

@app.route('/api/file/upload', methods=['POST'])
def file_upload():
    try:
        # 檢查是否有文件
        if 'file' not in request.files:
            return jsonify({"error": "沒有找到文件"}), 400
        
        file = request.files['file']
        if file.filename == '':
            return jsonify({"error": "沒有選擇文件"}), 400
        
        # 獲取相關參數
        file_type = request.form.get('file_type', 'unknown')
        related_id = request.form.get('related_id')
        
        # 讀取並編碼文件內容
        file_content = file.read()
        file_content_base64 = base64.b64encode(file_content).decode('utf-8')
        
        # 創建消息
        message = create_file_upload_message(
            file.filename,
            file_content_base64,
            file_type,
            related_id
        )
        
        # 連接 RabbitMQ 並發送消息
        connection, channel = initialize_rabbitmq()
        result = send_message(channel, 'file.upload', message)
        connection.close()
        
        if result:
            return jsonify({
                "message": "文件已上傳",
                "file_name": file.filename,
                "file_size": len(file_content),
                "message_id": message['message_id']
            })
        else:
            return jsonify({"error": "發送文件失敗"}), 500
    
    except Exception as e:
        return jsonify({"error": str(e)}), 500

@app.route('/api/message/custom', methods=['POST'])
def custom_message():
    try:
        data = request.json
        
        # 檢查必要參數
        if 'routing_key' not in data or 'payload' not in data:
            return jsonify({"error": "缺少必要參數: routing_key 和 payload"}), 400
        
        # 創建自定義消息
        message = create_message(
            data.get('message_type', 'custom'),
            data['payload'],
            data.get('correlation_id')
        )
        
        # 連接 RabbitMQ 並發送消息
        connection, channel = initialize_rabbitmq()
        result = send_message(channel, data['routing_key'], message)
        connection.close()
        
        if result:
            return jsonify({
                "message": "自定義消息已發送",
                "message_id": message['message_id'],
                "routing_key": data['routing_key']
            })
        else:
            return jsonify({"error": "發送自定義消息失敗"}), 500
    
    except Exception as e:
        return jsonify({"error": str(e)}), 500

if __name__ == '__main__':
    port = int(os.environ.get('PORT', 5000))
    app.run(host='0.0.0.0', port=port, debug=True)